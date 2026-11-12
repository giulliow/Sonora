mod audio;

use std::sync::Mutex;

use audio::{Engine, InputDevice, InputInfo, Take};
use serde::{Deserialize, Serialize};
use sonora_dsp::calibration::{
    analyze_loud, analyze_silence, analyze_speech, recommend, LoudReport, Recommendation,
    SilenceReport, SpeechReport,
};
use sonora_dsp::level::peak_dbfs;
use sonora_dsp::loudness::integrated_lufs;
use sonora_dsp::process::apply_gain_and_limit;
use tauri::{Emitter, Manager, State};

#[derive(Default)]
struct Calibration {
    silence: Option<SilenceReport>,
    speech: Option<SpeechReport>,
    loud: Option<LoudReport>,
}

struct AppState {
    engine: Engine,
    calibration: Mutex<Calibration>,
    last_take: Mutex<Option<Take>>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CalibrationStep {
    Silence,
    Speech,
    Loud,
}

#[derive(Debug, Clone, Serialize)]
struct TakeSummary {
    seconds: f32,
    peak_dbfs: f32,
    loudness_lufs: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "step", rename_all = "snake_case")]
enum StepResult {
    Silence(SilenceReport),
    Speech(SpeechReport),
    Loud(LoudReport),
}

fn summarize(take: &Take) -> TakeSummary {
    TakeSummary {
        seconds: take.samples.len() as f32 / take.sample_rate as f32,
        peak_dbfs: peak_dbfs(&take.samples),
        loudness_lufs: integrated_lufs(&take.samples, take.sample_rate),
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

/// Grava um trecho para "gravar e ouvir de volta".
#[tauri::command]
async fn record_take(state: State<'_, AppState>, seconds: f32) -> Result<TakeSummary, String> {
    let engine = state.engine.clone();
    let take = blocking(move || engine.record_blocking(seconds)).await?;
    let summary = summarize(&take);
    *state.last_take.lock().unwrap() = Some(take);
    Ok(summary)
}

/// Toca a última gravação. Com `gain_db`, aplica ganho + limitador
/// para a pessoa ouvir "como vai ficar".
#[tauri::command]
async fn play_take(state: State<'_, AppState>, gain_db: Option<f32>) -> Result<f32, String> {
    let take = state
        .last_take
        .lock()
        .unwrap()
        .clone()
        .ok_or("grave alguma coisa primeiro")?;
    let take = match gain_db {
        Some(g) => Take {
            samples: apply_gain_and_limit(&take.samples, g, take.sample_rate),
            sample_rate: take.sample_rate,
        },
        None => take,
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

/// Um passo da calibração: grava e analisa.
#[tauri::command]
async fn calibrate_step(
    state: State<'_, AppState>,
    step: CalibrationStep,
    seconds: f32,
) -> Result<StepResult, String> {
    let engine = state.engine.clone();
    let take = blocking(move || engine.record_blocking(seconds)).await?;
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
            *state.last_take.lock().unwrap() = Some(take);
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

#[tauri::command]
fn calibration_result(state: State<'_, AppState>) -> Result<Recommendation, String> {
    let cal = state.calibration.lock().unwrap();
    let silence = cal.silence.ok_or("faça o teste de silêncio primeiro")?;
    let speech = cal.speech.ok_or("faça o teste de voz normal primeiro")?;
    Ok(recommend(&silence, &speech, cal.loud.as_ref()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let engine = Engine::new(Box::new(move |snap| {
                let _ = handle.emit("meter", snap);
            }));
            app.manage(AppState {
                engine,
                calibration: Mutex::new(Calibration::default()),
                last_take: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
