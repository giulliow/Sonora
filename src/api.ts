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
}

export type Warning =
  | "no_speech_detected"
  | "too_far_from_mic"
  | "needs_more_input_gain"
  | "noisy_room"
  | "input_clipping";

export interface Recommendation {
  gain_db: number;
  gate_threshold_dbfs: number;
  voice_to_noise_db: number | null;
  limiter_will_engage: boolean;
  warnings: Warning[];
}

export type CalibrationStep = "silence" | "speech" | "loud";

export type StepResult =
  | { step: "silence"; noise_floor_dbfs: number }
  | { step: "speech"; loudness_lufs: number | null; peak_dbfs: number }
  | { step: "loud"; peak_dbfs: number; clipped: boolean };

export const api = {
  listInputs: () => invoke<InputDevice[]>("list_inputs"),
  startInput: (name: string | null) => invoke<InputInfo>("start_input", { name }),
  stopInput: () => invoke<void>("stop_input"),
  recordTake: (seconds: number) => invoke<TakeSummary>("record_take", { seconds }),
  playTake: (gainDb: number | null) => invoke<number>("play_take", { gainDb }),
  stopPlayback: () => invoke<void>("stop_playback"),
  isPlaying: () => invoke<boolean>("is_playing"),
  calibrateStep: (step: CalibrationStep, seconds: number) =>
    invoke<StepResult>("calibrate_step", { step, seconds }),
  calibrationResult: () => invoke<Recommendation>("calibration_result"),
  onMeter: (cb: (m: MeterSnapshot) => void): Promise<UnlistenFn> =>
    listen<MeterSnapshot>("meter", (e) => cb(e.payload)),
};

export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
