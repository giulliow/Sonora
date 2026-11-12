mod audio;
mod eqapo;
mod settings;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use audio::{Engine, InputDevice, InputInfo, Take};
use eqapo::EqApoStatus;
use serde::{Deserialize, Serialize};
use settings::{settings_path, Settings};
use sonora_dsp::calibration::{
    analyze_loud, analyze_silence, analyze_speech, recommend, LoudReport, Recommendation,
    SilenceReport, SpeechReport,
};
use sonora_dsp::chain::{render, ChainParams};
use sonora_dsp::level::peak_dbfs;
use sonora_dsp::loudness::integrated_lufs;
use sonora_dsp::presets::{suggest_simple, SimpleControls};
use tauri::{Emitter, Manager, State};

const EQAPO_DOWNLOAD_URL: &str =
    "https://sourceforge.net/projects/equalizerapo/files/latest/download";
/// Tempo para o Equalizer APO perceber uma mudança na configuração.
const EQAPO_RELOAD: Duration = Duration::from_millis(1500);

#[derive(Default)]
struct Calibration {
    silence: Option<SilenceReport>,
    speech: Option<SpeechReport>,
    loud: Option<LoudReport>,
}

struct LastTake {
    take: Take,
    /// Gravado com o tratamento do Windows já ligado.
    processed_by_windows: bool,
}

struct AppState {
    engine: Engine,
    calibration: Mutex<Calibration>,
    last_take: Mutex<Option<LastTake>>,
    settings: Mutex<Settings>,
    settings_file: PathBuf,
    plugin_path: PathBuf,
}

impl AppState {
    fn eqapo(&self) -> EqApoStatus {
        eqapo::status(&self.plugin_path)
    }

    fn view(&self, apply_error: Option<String>) -> AppView {
        let settings = self.settings.lock().unwrap().clone();
        let rec = settings
            .recommendation
            .as_ref()
            .filter(|_| settings.use_calibration);
        AppView {
            effective: settings.effective(),
            simple_params: settings.simple_params(),
            suggested_simple: suggest_simple(settings.mic_type, rec),
            settings,
            eqapo: self.eqapo(),
            apply_error,
        }
    }

    /// Escreve a configuração no Windows (se o Sonora já estiver ativado lá).
    fn push_to_windows(&self, force_link: bool) -> Option<String> {
        let st = self.eqapo();
        if !st.installed || !(st.linked || force_link) {
            return None;
        }
        let params = self.settings.lock().unwrap().effective();
        eqapo::apply(&self.plugin_path, &params).err()
    }
}

#[derive(Debug, Clone, Serialize)]
struct AppView {
    settings: Settings,
    /// A cadeia que está valendo.
    effective: ChainParams,
    /// O que os controles simples produzem (base do modo avançado).
    simple_params: ChainParams,
    /// O que o Sonora sugere para os controles simples.
    suggested_simple: SimpleControls,
    eqapo: EqApoStatus,
    apply_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CalibrationStep {
    Silence,
    Speech,
    Loud,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PlayMode {
    /// Como foi gravado.
    Raw,
    /// Com a configuração atual aplicada no app (prévia).
    Preview,
}

#[derive(Debug, Clone, Serialize)]
struct TakeSummary {
    seconds: f32,
    peak_dbfs: f32,
    loudness_lufs: Option<f32>,
    processed_by_windows: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "step", rename_all = "snake_case")]
enum StepResult {
    Silence(SilenceReport),
    Speech(SpeechReport),
    Loud(LoudReport),
}

#[derive(Debug, Clone, Serialize)]
struct CalibrationOutcome {
    recommendation: Recommendation,
    /// Controles simples sugeridos com esta calibração.
    suggested_simple: SimpleControls,
}

fn summarize(t: &LastTake) -> TakeSummary {
    TakeSummary {
        seconds: t.take.samples.len() as f32 / t.take.sample_rate as f32,
        peak_dbfs: peak_dbfs(&t.take.samples),
        loudness_lufs: integrated_lufs(&t.take.samples, t.take.sample_rate),
        processed_by_windows: t.processed_by_windows,
    }
}

/// Roda trabalho que bloqueia (gravar por N segundos) fora da thread da tela.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

// ------------------------------------------------------------ estado e configurações

#[tauri::command]
fn get_state(state: State<'_, AppState>) -> AppView {
    state.view(None)
}

/// Salva tudo e, se o Sonora já estiver ativado no Windows, aplica na hora.
#[tauri::command]
fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<AppView, String> {
    settings.save(&state.settings_file)?;
    *state.settings.lock().unwrap() = settings;
    let err = state.push_to_windows(false);
    Ok(state.view(err))
}

/// Primeira ativação: liga o Sonora na configuração do Equalizer APO.
#[tauri::command]
fn activate_windows(state: State<'_, AppState>) -> Result<AppView, String> {
    match state.push_to_windows(true) {
        Some(e) => Err(e),
        None => Ok(state.view(None)),
    }
}

#[tauri::command]
fn open_eqapo_download() -> Result<(), String> {
    tauri_plugin_opener::open_url(EQAPO_DOWNLOAD_URL, None::<&str>).map_err(|e| e.to_string())
}

/// Abre o "Configurator" do Equalizer APO (pede permissão de administrador).
#[tauri::command]
fn open_eqapo_configurator(state: State<'_, AppState>) -> Result<(), String> {
    let path = state
        .eqapo()
        .configurator
        .ok_or("não achei o Configurator do Equalizer APO")?;
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| e.to_string())
}

// ------------------------------------------------------------ microfone

#[tauri::command]
async fn list_inputs(state: State<'_, AppState>) -> Result<Vec<InputDevice>, String> {
    let engine = state.engine.clone();
    blocking(move || engine.list_inputs()).await
}

#[tauri::command]
async fn start_input(
    state: State<'_, AppState>,
    name: Option<String>,
) -> Result<InputInfo, String> {
    let engine = state.engine.clone();
    *state.calibration.lock().unwrap() = Calibration::default();
    blocking(move || engine.start_input(name)).await
}

#[tauri::command]
async fn stop_input(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.engine.clone();
    blocking(move || engine.stop_input()).await
}

// ------------------------------------------------------------ gravar e ouvir

/// Grava como os outros ouvem (com o tratamento do Windows, se estiver ligado).
#[tauri::command]
async fn record_take(state: State<'_, AppState>, seconds: f32) -> Result<TakeSummary, String> {
    let engine = state.engine.clone();
    let processed_by_windows = state.eqapo().processing_on;
    let take = blocking(move || engine.record_blocking(seconds)).await?;
    let last = LastTake {
        take,
        processed_by_windows,
    };
    let summary = summarize(&last);
    *state.last_take.lock().unwrap() = Some(last);
    Ok(summary)
}

#[tauri::command]
async fn play_take(state: State<'_, AppState>, mode: PlayMode) -> Result<f32, String> {
    let take = {
        let guard = state.last_take.lock().unwrap();
        let last = guard.as_ref().ok_or("grave alguma coisa primeiro")?;
        if mode == PlayMode::Preview && last.processed_by_windows {
            return Err("esta gravação já tem o tratamento do Windows; toque normalmente".into());
        }
        last.take.clone()
    };
    let take = match mode {
        PlayMode::Raw => take,
        PlayMode::Preview => {
            let mut params = state.settings.lock().unwrap().effective();
            params.enabled = true;
            Take {
                samples: render(&take.samples, params, take.sample_rate),
                sample_rate: take.sample_rate,
            }
        }
    };
    let engine = state.engine.clone();
    blocking(move || engine.play(take)).await
}

#[tauri::command]
async fn stop_playback(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.engine.clone();
    blocking(move || engine.stop_playback()).await
}

#[tauri::command]
fn is_playing(state: State<'_, AppState>) -> bool {
    state.engine.is_playing()
}

// ------------------------------------------------------------ calibração

/// Um passo da calibração. Mede o microfone "cru": se o tratamento do
/// Windows estiver ligado, desliga por alguns segundos e religa depois.
#[tauri::command]
async fn calibrate_step(
    state: State<'_, AppState>,
    step: CalibrationStep,
    seconds: f32,
) -> Result<StepResult, String> {
    let st = state.eqapo();
    let bypass = st.linked && st.processing_on;
    if bypass {
        let mut off = state.settings.lock().unwrap().effective();
        off.enabled = false;
        eqapo::apply(&state.plugin_path, &off)?;
        tokio_sleep(EQAPO_RELOAD).await;
    }

    let engine = state.engine.clone();
    let recorded = blocking(move || engine.record_blocking(seconds)).await;

    if bypass {
        if let Some(e) = state.push_to_windows(false) {
            return Err(format!(
                "a calibração terminou, mas não consegui religar o tratamento: {e}"
            ));
        }
    }
    let take = recorded?;

    let mut cal = state.calibration.lock().unwrap();
    let result = match step {
        CalibrationStep::Silence => {
            let r = analyze_silence(&take.samples, take.sample_rate);
            cal.silence = Some(r);
            StepResult::Silence(r)
        }
        CalibrationStep::Speech => {
            let r = analyze_speech(&take.samples, take.sample_rate);
            cal.speech = Some(r);
            // A frase lida também vira a gravação para comparar antes/depois.
            *state.last_take.lock().unwrap() = Some(LastTake {
                take,
                processed_by_windows: false,
            });
            StepResult::Speech(r)
        }
        CalibrationStep::Loud => {
            let r = analyze_loud(&take.samples);
            cal.loud = Some(r);
            StepResult::Loud(r)
        }
    };
    Ok(result)
}

async fn tokio_sleep(d: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}

/// Resultado da calibração. Não muda nada sozinho: a tela mostra e a
/// pessoa decide se usa (chamando `save_settings`).
#[tauri::command]
fn calibration_result(state: State<'_, AppState>) -> Result<CalibrationOutcome, String> {
    let cal = state.calibration.lock().unwrap();
    let silence = cal.silence.ok_or("faça o teste de silêncio primeiro")?;
    let speech = cal.speech.ok_or("faça o teste de voz normal primeiro")?;
    let recommendation = recommend(&silence, &speech, cal.loud.as_ref());
    let mic = state.settings.lock().unwrap().mic_type;
    Ok(CalibrationOutcome {
        suggested_simple: suggest_simple(mic, Some(&recommendation)),
        recommendation,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let engine = Engine::new(Box::new(move |snap| {
                let _ = handle.emit("meter", snap);
            }));
            let settings_file = settings_path(app.path().app_config_dir()?);
            let plugin_path = app
                .path()
                .resource_dir()
                .map(|d| d.join("sonora_vst.dll"))
                .unwrap_or_else(|_| PathBuf::from("sonora_vst.dll"));
            let settings = Settings::load(&settings_file);
            app.manage(AppState {
                engine,
                calibration: Mutex::new(Calibration::default()),
                last_take: Mutex::new(None),
                settings: Mutex::new(settings),
                settings_file,
                plugin_path,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_settings,
            activate_windows,
            open_eqapo_download,
            open_eqapo_configurator,
            list_inputs,
            start_input,
            stop_input,
            record_take,
            play_take,
            stop_playback,
            is_playing,
            calibrate_step,
            calibration_result,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Sonora");
}
