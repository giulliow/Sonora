//! Conversa com a placa de som: abre o microfone, mede, grava e toca.
//!
//! Os streams do `cpal` não podem trocar de thread em todas as plataformas,
//! então eles moram numa thread de áudio própria que recebe comandos.
//! O callback do microfone só converte para mono e joga num buffer circular
//! (sem travas, sem alocação). Uma thread de processamento lê esse buffer,
//! atualiza os medidores e grava quando pedido.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use serde::Serialize;
use sonora_dsp::level::{zone_for_peak, PeakMeter, Zone};
use sonora_dsp::loudness::LoudnessMeter;
use sonora_dsp::mic::{guess_from_name, MicGuess};
use sonora_dsp::resample::resample_linear;

#[derive(Debug, Clone, Serialize)]
pub struct InputDevice {
    pub name: String,
    pub is_default: bool,
    pub guess: Option<MicGuess>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputInfo {
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
}

/// O que a tela recebe ~20 vezes por segundo.
#[derive(Debug, Clone, Serialize)]
pub struct MeterSnapshot {
    pub peak_dbfs: f32,
    pub hold_dbfs: f32,
    pub momentary_lufs: Option<f32>,
    pub short_term_lufs: Option<f32>,
    pub zone: Zone,
    pub recording: bool,
}

/// Um trecho gravado, em mono.
#[derive(Debug, Clone)]
pub struct Take {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Default)]
struct Recording {
    active: bool,
    target_len: usize,
    samples: Vec<f32>,
}

#[derive(Default)]
struct Shared {
    recording: Mutex<Recording>,
    sample_rate: Mutex<Option<u32>>,
    playing: AtomicBool,
}

type Reply<T> = Sender<Result<T, String>>;

enum Cmd {
    StartInput(Option<String>, Reply<InputInfo>),
    StopInput(Reply<()>),
    Play(Take, Reply<f32>),
    StopPlayback(Reply<()>),
}

pub type MeterCallback = Box<dyn Fn(MeterSnapshot) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct Engine {
    tx: Sender<Cmd>,
    shared: Arc<Shared>,
}

impl Engine {
    pub fn new(on_meter: MeterCallback) -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared::default());
        let s = shared.clone();
        let on_meter: Arc<MeterCallback> = Arc::new(on_meter);
        thread::Builder::new()
            .name("sonora-audio".into())
            .spawn(move || audio_thread(rx, s, on_meter))
            .expect("criar thread de áudio");
        Self { tx, shared }
    }

    fn ask<T>(&self, make: impl FnOnce(Reply<T>) -> Cmd) -> Result<T, String> {
        let (rtx, rrx) = mpsc::channel();
        self.tx
            .send(make(rtx))
            .map_err(|_| "o motor de áudio parou".to_string())?;
        rrx.recv()
            .map_err(|_| "o motor de áudio parou".to_string())?
    }

    pub fn list_inputs(&self) -> Result<Vec<InputDevice>, String> {
        let host = cpal::default_host();
        let default_name = host.default_input_device().and_then(|d| d.name().ok());
        let devices = host
            .input_devices()
            .map_err(|e| format!("não foi possível listar os microfones: {e}"))?;
        let mut out: Vec<InputDevice> = devices
            .filter_map(|d| d.name().ok())
            .map(|name| InputDevice {
                is_default: Some(&name) == default_name.as_ref(),
                guess: guess_from_name(&name),
                name,
            })
            .collect();
        out.sort_by_key(|d| !d.is_default);
        Ok(out)
    }

    pub fn start_input(&self, name: Option<String>) -> Result<InputInfo, String> {
        self.ask(|r| Cmd::StartInput(name, r))
    }

    pub fn stop_input(&self) -> Result<(), String> {
        self.ask(Cmd::StopInput)
    }

    /// Grava `seconds` segundos do microfone aberto. Bloqueia até terminar.
    pub fn record_blocking(&self, seconds: f32) -> Result<Take, String> {
        let sample_rate = self
            .shared
            .sample_rate
            .lock()
            .unwrap()
            .ok_or("abra um microfone antes de gravar")?;
        let target_len = (seconds.clamp(0.5, 60.0) * sample_rate as f32) as usize;
        {
            let mut rec = self.shared.recording.lock().unwrap();
            rec.samples = Vec::with_capacity(target_len);
            rec.target_len = target_len;
            rec.active = true;
        }
        let deadline = Instant::now() + Duration::from_secs_f32(seconds + 3.0);
        loop {
            thread::sleep(Duration::from_millis(50));
            let mut rec = self.shared.recording.lock().unwrap();
            if !rec.active {
                return Ok(Take {
                    samples: std::mem::take(&mut rec.samples),
                    sample_rate,
                });
            }
            if Instant::now() > deadline {
                rec.active = false;
                return Err("o microfone parou de mandar som durante a gravação".into());
            }
        }
    }

    /// Toca um trecho no fone/caixa padrão. Devolve a duração em segundos.
    pub fn play(&self, take: Take) -> Result<f32, String> {
        self.ask(|r| Cmd::Play(take, r))
    }

    pub fn stop_playback(&self) -> Result<(), String> {
        self.ask(Cmd::StopPlayback)
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }
}

struct InputRunning {
    _stream: cpal::Stream,
    running: Arc<AtomicBool>,
}

impl Drop for InputRunning {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn audio_thread(rx: Receiver<Cmd>, shared: Arc<Shared>, on_meter: Arc<MeterCallback>) {
    let mut input: Option<InputRunning> = None;
    let mut output: Option<cpal::Stream> = None;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::StartInput(name, reply) => {
                drop(input.take()); // fecha o anterior antes de abrir outro
                *shared.sample_rate.lock().unwrap() = None;
                let res = open_input(name, shared.clone(), on_meter.clone()).map(|(run, info)| {
                    *shared.sample_rate.lock().unwrap() = Some(info.sample_rate);
                    input = Some(run);
                    info
                });
                let _ = reply.send(res);
            }
            Cmd::StopInput(reply) => {
                drop(input.take());
                *shared.sample_rate.lock().unwrap() = None;
                let _ = reply.send(Ok(()));
            }
            Cmd::Play(take, reply) => {
                drop(output.take());
                let res = open_output(take, shared.clone()).map(|(stream, secs)| {
                    output = Some(stream);
                    secs
                });
                let _ = reply.send(res);
            }
            Cmd::StopPlayback(reply) => {
                drop(output.take());
                shared.playing.store(false, Ordering::Relaxed);
                let _ = reply.send(Ok(()));
            }
        }
    }
}

fn find_input(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device, String> {
    match name {
        None => host
            .default_input_device()
            .ok_or_else(|| "nenhum microfone encontrado".to_string()),
        Some(wanted) => host
            .input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().map(|n| n == wanted).unwrap_or(false))
            .ok_or_else(|| format!("microfone \"{wanted}\" não encontrado (foi desconectado?)")),
    }
}

fn open_input(
    name: Option<String>,
    shared: Arc<Shared>,
    on_meter: Arc<MeterCallback>,
) -> Result<(InputRunning, InputInfo), String> {
    let host = cpal::default_host();
    let device = find_input(&host, name.as_deref())?;
    let dev_name = device.name().unwrap_or_else(|_| "Microfone".into());
    let config = device
        .default_input_config()
        .map_err(|e| format!("não foi possível ler as configurações do microfone: {e}"))?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    let ch = channels as usize;

    // 2 segundos de folga entre o callback e a thread de processamento.
    let (prod, cons) = rtrb::RingBuffer::<f32>::new(sample_rate as usize * 2);
    let err_fn = |e| eprintln!("erro no microfone: {e}");
    let stream_config: cpal::StreamConfig = config.clone().into();

    let stream = match config.sample_format() {
        SampleFormat::F32 => build_input::<f32>(&device, &stream_config, ch, prod, err_fn),
        SampleFormat::I16 => build_input::<i16>(&device, &stream_config, ch, prod, err_fn),
        SampleFormat::I32 => build_input::<i32>(&device, &stream_config, ch, prod, err_fn),
        SampleFormat::U16 => build_input::<u16>(&device, &stream_config, ch, prod, err_fn),
        other => return Err(format!("formato de áudio não suportado: {other}")),
    }
    .map_err(|e| format!("não foi possível abrir o microfone: {e}"))?;
    stream
        .play()
        .map_err(|e| format!("não foi possível iniciar o microfone: {e}"))?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    thread::Builder::new()
        .name("sonora-meter".into())
        .spawn(move || process_loop(cons, sample_rate, r, shared, on_meter))
        .map_err(|e| e.to_string())?;

    Ok((
        InputRunning {
            _stream: stream,
            running,
        },
        InputInfo {
            name: dev_name,
            sample_rate,
            channels,
        },
    ))
}

fn build_input<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    mut prod: rtrb::Producer<f32>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let inv = 1.0 / channels.max(1) as f32;
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            for frame in data.chunks_exact(channels.max(1)) {
                let mono = frame.iter().map(|s| s.to_sample::<f32>()).sum::<f32>() * inv;
                // Se o buffer encher (thread travada), descartamos: nunca
                // bloquear o callback de áudio.
                let _ = prod.push(mono);
            }
        },
        err_fn,
        None,
    )
}

fn process_loop(
    mut cons: rtrb::Consumer<f32>,
    sample_rate: u32,
    running: Arc<AtomicBool>,
    shared: Arc<Shared>,
    on_meter: Arc<MeterCallback>,
) {
    let mut peak = PeakMeter::default();
    let mut loud = LoudnessMeter::new(sample_rate);
    let mut buf: Vec<f32> = Vec::with_capacity(sample_rate as usize / 10);
    let emit_every = (sample_rate / 20) as usize; // 50 ms
    let mut since_emit = 0usize;

    while running.load(Ordering::Relaxed) {
        let n = cons.slots();
        if n == 0 {
            thread::sleep(Duration::from_millis(5));
            continue;
        }
        buf.clear();
        if let Ok(chunk) = cons.read_chunk(n) {
            let (a, b) = chunk.as_slices();
            buf.extend_from_slice(a);
            buf.extend_from_slice(b);
            chunk.commit_all();
        }

        loud.push(&buf);
        peak.update(&buf, buf.len() as f32 / sample_rate as f32);

        let recording = {
            let mut rec = shared.recording.lock().unwrap();
            if rec.active {
                let room = rec.target_len.saturating_sub(rec.samples.len());
                let take = room.min(buf.len());
                rec.samples.extend_from_slice(&buf[..take]);
                if rec.samples.len() >= rec.target_len {
                    rec.active = false;
                }
            }
            rec.active
        };

        since_emit += buf.len();
        if since_emit >= emit_every {
            since_emit = 0;
            on_meter(MeterSnapshot {
                peak_dbfs: peak.current_db(),
                hold_dbfs: peak.hold_db(),
                momentary_lufs: loud.momentary(),
                short_term_lufs: loud.short_term(),
                zone: zone_for_peak(peak.current_db()),
                recording,
            });
        }
    }
}

fn open_output(take: Take, shared: Arc<Shared>) -> Result<(cpal::Stream, f32), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("nenhuma saída de som encontrada (fone ou caixa)")?;
    let config = device
        .default_output_config()
        .map_err(|e| format!("não foi possível ler a saída de som: {e}"))?;
    let out_rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    let samples = Arc::new(resample_linear(&take.samples, take.sample_rate, out_rate));
    let secs = samples.len() as f32 / out_rate as f32;
    let stream_config: cpal::StreamConfig = config.clone().into();
    let err_fn = |e| eprintln!("erro na saída de som: {e}");

    shared.playing.store(true, Ordering::Relaxed);
    let stream = match config.sample_format() {
        SampleFormat::F32 => build_output::<f32>(
            &device,
            &stream_config,
            channels,
            samples,
            shared.clone(),
            err_fn,
        ),
        SampleFormat::I16 => build_output::<i16>(
            &device,
            &stream_config,
            channels,
            samples,
            shared.clone(),
            err_fn,
        ),
        SampleFormat::I32 => build_output::<i32>(
            &device,
            &stream_config,
            channels,
            samples,
            shared.clone(),
            err_fn,
        ),
        SampleFormat::U16 => build_output::<u16>(
            &device,
            &stream_config,
            channels,
            samples,
            shared.clone(),
            err_fn,
        ),
        other => {
            shared.playing.store(false, Ordering::Relaxed);
            return Err(format!("formato de saída não suportado: {other}"));
        }
    }
    .map_err(|e| {
        shared.playing.store(false, Ordering::Relaxed);
        format!("não foi possível abrir a saída de som: {e}")
    })?;
    stream
        .play()
        .map_err(|e| format!("não foi possível tocar: {e}"))?;
    Ok((stream, secs))
}

fn build_output<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    samples: Arc<Vec<f32>>,
    shared: Arc<Shared>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample + FromSample<f32>,
{
    let mut pos = 0usize;
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for frame in data.chunks_mut(channels.max(1)) {
                let s = samples.get(pos).copied().unwrap_or(0.0);
                pos += 1;
                let v = T::from_sample(s);
                frame.iter_mut().for_each(|x| *x = v);
            }
            if pos >= samples.len() {
                shared.playing.store(false, Ordering::Relaxed);
            }
        },
        err_fn,
        None,
    )
}
