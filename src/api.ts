// Ponte entre a tela e o motor de áudio em Rust.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type MicType = "dynamic" | "condenser" | "headset" | "built_in" | "lavalier";
export type Zone = "low" | "good" | "hot" | "clip";

export interface MicGuess {
  mic_type: MicType;
  model: string | null;
  confidence: "high" | "low";
}

export interface InputDevice {
  name: string;
  is_default: boolean;
  guess: MicGuess | null;
}

export interface InputInfo {
  name: string;
  sample_rate: number;
  channels: number;
}

export interface MeterSnapshot {
  peak_dbfs: number;
  hold_dbfs: number;
  momentary_lufs: number | null;
  short_term_lufs: number | null;
  zone: Zone;
  recording: boolean;
}

export interface TakeSummary {
  seconds: number;
  peak_dbfs: number;
  loudness_lufs: number | null;
  /** Gravado com o tratamento do Windows já ligado. */
  processed_by_windows: boolean;
}

export type Warning =
  | "no_speech_detected"
  | "too_far_from_mic"
  | "needs_more_input_gain"
  | "noisy_room"
  | "input_clipping";

export interface Recommendation {
  gain_db: number;
  gate_threshold_dbfs: number | null;
  mic_mutes_itself: boolean;
  loud_peak_after_gain_dbfs: number | null;
  voice_to_noise_db: number | null;
  limiter_will_engage: boolean;
  warnings: Warning[];
}

export interface ChainParams {
  enabled: boolean;
  input_gain_db: number;
  hpf_on: boolean;
  hpf_hz: number;
  denoise_on: boolean;
  gate_on: boolean;
  gate_threshold_db: number;
  gate_range_db: number;
  gate_hold_ms: number;
  gate_release_ms: number;
  eq_on: boolean;
  eq_low_db: number;
  eq_presence_db: number;
  eq_air_db: number;
  deess_on: boolean;
  deess_threshold_db: number;
  deess_max_reduction_db: number;
  comp_on: boolean;
  comp_threshold_db: number;
  comp_ratio: number;
  comp_attack_ms: number;
  comp_release_ms: number;
  comp_makeup_db: number;
  limiter_on: boolean;
  limiter_ceiling_db: number;
}

export interface SimpleControls {
  volume_db: number;
  cleanup: number;
  tone: number;
  evenness: number;
}

export type Mode = "simple" | "advanced";

export interface Settings {
  device_name: string | null;
  mic_type: MicType | null;
  recommendation: Recommendation | null;
  use_calibration: boolean;
  mode: Mode;
  simple: SimpleControls;
  advanced: ChainParams | null;
  enabled: boolean;
}

export interface EqApoStatus {
  installed: boolean;
  config_dir: string | null;
  configurator: string | null;
  plugin_found: boolean;
  linked: boolean;
  has_other_settings: boolean;
  processing_on: boolean;
}

export interface AppView {
  settings: Settings;
  effective: ChainParams;
  simple_params: ChainParams;
  suggested_simple: SimpleControls;
  eqapo: EqApoStatus;
  apply_error: string | null;
}

export interface CalibrationOutcome {
  recommendation: Recommendation;
  suggested_simple: SimpleControls;
}

export type CalibrationStep = "silence" | "speech" | "loud";

export type StepResult =
  | { step: "silence"; noise_floor_dbfs: number }
  | { step: "speech"; loudness_lufs: number | null; peak_dbfs: number }
  | { step: "loud"; peak_dbfs: number; clipped: boolean };

export const api = {
  getState: () => invoke<AppView>("get_state"),
  saveSettings: (settings: Settings) => invoke<AppView>("save_settings", { settings }),
  activateWindows: () => invoke<AppView>("activate_windows"),
  openEqapoDownload: () => invoke<void>("open_eqapo_download"),
  openEqapoConfigurator: () => invoke<void>("open_eqapo_configurator"),
  listInputs: () => invoke<InputDevice[]>("list_inputs"),
  startInput: (name: string | null) => invoke<InputInfo>("start_input", { name }),
  stopInput: () => invoke<void>("stop_input"),
  recordTake: (seconds: number) => invoke<TakeSummary>("record_take", { seconds }),
  playTake: (mode: "raw" | "preview") => invoke<number>("play_take", { mode }),
  stopPlayback: () => invoke<void>("stop_playback"),
  isPlaying: () => invoke<boolean>("is_playing"),
  calibrateStep: (step: CalibrationStep, seconds: number) =>
    invoke<StepResult>("calibrate_step", { step, seconds }),
  calibrationResult: () => invoke<CalibrationOutcome>("calibration_result"),
  onMeter: (cb: (m: MeterSnapshot) => void): Promise<UnlistenFn> =>
    listen<MeterSnapshot>("meter", (e) => cb(e.payload)),
};

export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
