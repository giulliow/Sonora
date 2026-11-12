//! Calibração smart: três testes curtos e uma recomendação.
//!
//! 1. Silêncio: mede o ruído de fundo do quarto.
//! 2. Voz normal: mede o volume da fala e calcula quanto ganho falta.
//! 3. Voz alta: confere se o pico estoura.

use crate::level::{peak_dbfs, rms_dbfs};
use crate::loudness::integrated_lufs;
use serde::{Deserialize, Serialize};

/// Volume alvo de conversa. -18 LUFS soa "cheio" no Discord sem estourar.
pub const TARGET_LUFS: f32 = -18.0;
/// Teto do limitador.
pub const CEILING_DBFS: f32 = -1.0;
/// Acima disso de ganho por software, o ruído de fundo sobe junto e fica feio.
pub const MAX_COMFORTABLE_SOFTWARE_GAIN_DB: f32 = 20.0;
/// Abaixo disso o "silêncio" é zero digital, não o ruído de um quarto real.
pub const DIGITAL_SILENCE_DBFS: f32 = -90.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SilenceReport {
    /// Ruído de fundo (RMS típico) em dBFS.
    pub noise_floor_dbfs: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SpeechReport {
    /// Volume percebido da fala. `None` = não ouvimos fala.
    pub loudness_lufs: Option<f32>,
    pub peak_dbfs: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LoudReport {
    pub peak_dbfs: f32,
    /// Bateu no teto digital antes mesmo do app: distorção que não tem conserto depois.
    pub clipped: bool,
}

/// Mede o ruído de fundo. Usa a mediana de blocos de 100 ms, para que um
/// clique ou uma tossida não estrague a medida.
pub fn analyze_silence(mono: &[f32], sample_rate: u32) -> SilenceReport {
    let block = (sample_rate as usize / 10).max(1);
    let mut levels: Vec<f32> = mono
        .chunks(block)
        .filter(|c| c.len() == block)
        .map(rms_dbfs)
        .collect();
    if levels.is_empty() {
        return SilenceReport {
            noise_floor_dbfs: rms_dbfs(mono),
        };
    }
    levels.sort_by(|a, b| a.total_cmp(b));
    SilenceReport {
        noise_floor_dbfs: levels[levels.len() / 2],
    }
}

pub fn analyze_speech(mono: &[f32], sample_rate: u32) -> SpeechReport {
    SpeechReport {
        loudness_lufs: integrated_lufs(mono, sample_rate),
        peak_dbfs: peak_dbfs(mono),
    }
}

pub fn analyze_loud(mono: &[f32]) -> LoudReport {
    // 3 amostras seguidas grudadas no máximo = distorção de verdade.
    let mut run = 0;
    let mut clipped = false;
    for s in mono {
        if s.abs() >= 0.999 {
            run += 1;
            if run >= 3 {
                clipped = true;
                break;
            }
        } else {
            run = 0;
        }
    }
    LoudReport {
        peak_dbfs: peak_dbfs(mono),
        clipped,
    }
}

/// Avisos em linguagem de máquina. A tela transforma em frases simples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Warning {
    /// Não ouvimos fala no teste de voz normal.
    NoSpeechDetected,
    /// Fala muito baixa: provavelmente longe do microfone.
    TooFarFromMic,
    /// Precisa de tanto ganho que vale subir o ganho no próprio mic/sistema.
    NeedsMoreInputGain,
    /// O ruído de fundo está perto demais da voz.
    NoisyRoom,
    /// A voz alta estourou antes do app: abaixar o ganho de entrada ou se afastar.
    InputClipping,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    /// Quanto subir (+) ou descer (-) o volume, em dB.
    pub gain_db: f32,
    /// A partir de que nível a porta de ruído abre (dBFS, antes do ganho).
    /// `None` quando o microfone já entrega silêncio total nas pausas
    /// (alguns, como o Shure MV7, têm porta de ruído própria).
    pub gate_threshold_dbfs: Option<f32>,
    /// O silêncio veio como "zero digital": o mic já corta o ruído sozinho.
    pub mic_mutes_itself: bool,
    /// Pico da voz alta depois do ajuste de volume (dBFS). Acima de 0
    /// significa que só o limitador segura: vale usar o compressor.
    pub loud_peak_after_gain_dbfs: Option<f32>,
    /// Distância entre voz e ruído de fundo, em dB. Acima de 30 é ótimo.
    pub voice_to_noise_db: Option<f32>,
    /// O limitador vai precisar trabalhar nos gritos (normal, só informativo).
    pub limiter_will_engage: bool,
    pub warnings: Vec<Warning>,
}

pub fn recommend(
    silence: &SilenceReport,
    speech: &SpeechReport,
    loud: Option<&LoudReport>,
) -> Recommendation {
    let mut warnings = Vec::new();
    let noise = silence.noise_floor_dbfs;
    let mic_mutes_itself = noise <= DIGITAL_SILENCE_DBFS;

    let Some(lufs) = speech.loudness_lufs.filter(|l| *l > -70.0) else {
        return Recommendation {
            gain_db: 0.0,
            gate_threshold_dbfs: (!mic_mutes_itself).then_some(noise + 6.0),
            mic_mutes_itself,
            loud_peak_after_gain_dbfs: None,
            voice_to_noise_db: None,
            limiter_will_engage: false,
            warnings: vec![Warning::NoSpeechDetected],
        };
    };

    let gain_db = (TARGET_LUFS - lufs).clamp(-20.0, 30.0);
    let voice_to_noise = lufs - noise;

    if lufs < -38.0 {
        warnings.push(Warning::TooFarFromMic);
    }
    if gain_db > MAX_COMFORTABLE_SOFTWARE_GAIN_DB {
        warnings.push(Warning::NeedsMoreInputGain);
    }
    if !mic_mutes_itself && voice_to_noise < 25.0 {
        warnings.push(Warning::NoisyRoom);
    }

    let loud_peak = loud.map(|l| l.peak_dbfs).unwrap_or(speech.peak_dbfs);
    if loud.is_some_and(|l| l.clipped) {
        warnings.push(Warning::InputClipping);
    }

    // Porta abre entre o ruído e a voz, mais perto do ruído, para não
    // cortar o começo das palavras. Sempre pelo menos 6 dB acima do ruído.
    let gap = (voice_to_noise * 0.35).clamp(6.0, 18.0);

    let round = |v: f32| (v * 10.0).round() / 10.0;
    Recommendation {
        gain_db: round(gain_db),
        gate_threshold_dbfs: (!mic_mutes_itself).then(|| round(noise + gap)),
        mic_mutes_itself,
        loud_peak_after_gain_dbfs: loud.map(|l| round(l.peak_dbfs + gain_db)),
        voice_to_noise_db: (!mic_mutes_itself).then(|| round(voice_to_noise)),
        limiter_will_engage: loud_peak + gain_db > CEILING_DBFS,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amp_from_db;
    use crate::test_util::{noise, sine};

    const SR: u32 = 48_000;

    #[test]
    fn silence_uses_median() {
        let mut s = noise(amp_from_db(-60.0), 3.0, SR);
        // um "clique" alto no meio não deve mudar a medida
        for x in s.iter_mut().skip(10_000).take(2_000) {
            *x = 0.8;
        }
        let r = analyze_silence(&s, SR);
        assert!(
            r.noise_floor_dbfs < -55.0 && r.noise_floor_dbfs > -70.0,
            "{r:?}"
        );
    }

    #[test]
    fn quiet_voice_gets_positive_gain() {
        let silence = analyze_silence(&noise(amp_from_db(-70.0), 2.0, SR), SR);
        // "voz" a ~-30 LUFS
        let speech = analyze_speech(&sine(1000.0, amp_from_db(-27.0), 5.0, SR), SR);
        let r = recommend(&silence, &speech, None);
        assert!((r.gain_db - 12.0).abs() < 0.5, "{r:?}");
        assert!(r.warnings.is_empty(), "{r:?}");
        assert!(r.gate_threshold_dbfs.unwrap() > silence.noise_floor_dbfs + 5.9);
    }

    #[test]
    fn very_quiet_voice_warns_distance_and_gain() {
        let silence = analyze_silence(&noise(amp_from_db(-80.0), 2.0, SR), SR);
        let speech = analyze_speech(&sine(1000.0, amp_from_db(-42.0), 5.0, SR), SR);
        let r = recommend(&silence, &speech, None);
        assert!(r.warnings.contains(&Warning::TooFarFromMic));
        assert!(r.warnings.contains(&Warning::NeedsMoreInputGain));
    }

    #[test]
    fn noisy_room_warns() {
        let silence = analyze_silence(&noise(amp_from_db(-35.0), 2.0, SR), SR);
        let speech = analyze_speech(&sine(1000.0, amp_from_db(-17.0), 5.0, SR), SR);
        let r = recommend(&silence, &speech, None);
        assert!(r.warnings.contains(&Warning::NoisyRoom), "{r:?}");
    }

    #[test]
    fn clipping_detected() {
        let loud = analyze_loud(&[0.2, 1.0, 1.0, 1.0, 0.3]);
        assert!(loud.clipped);
        let silence = SilenceReport {
            noise_floor_dbfs: -70.0,
        };
        let speech = SpeechReport {
            loudness_lufs: Some(-18.0),
            peak_dbfs: -6.0,
        };
        let r = recommend(&silence, &speech, Some(&loud));
        assert!(r.warnings.contains(&Warning::InputClipping));
        assert!(r.limiter_will_engage);
    }

    #[test]
    fn digital_silence_means_mic_mutes_itself() {
        // Caso real do Shure MV7: pausa = zero digital.
        let silence = analyze_silence(&vec![0.0; SR as usize * 2], SR);
        let speech = SpeechReport {
            loudness_lufs: Some(-36.8),
            peak_dbfs: -20.0,
        };
        let loud = LoudReport {
            peak_dbfs: -1.7,
            clipped: false,
        };
        let r = recommend(&silence, &speech, Some(&loud));
        assert!(r.mic_mutes_itself);
        assert_eq!(r.gate_threshold_dbfs, None);
        assert_eq!(r.voice_to_noise_db, None);
        assert!(!r.warnings.contains(&Warning::NoisyRoom));
        assert!((r.loud_peak_after_gain_dbfs.unwrap() - 17.1).abs() < 0.2);
    }

    #[test]
    fn no_speech() {
        let silence = SilenceReport {
            noise_floor_dbfs: -70.0,
        };
        let speech = analyze_speech(&vec![0.0; SR as usize], SR);
        let r = recommend(&silence, &speech, None);
        assert_eq!(r.warnings, vec![Warning::NoSpeechDetected]);
    }
}
