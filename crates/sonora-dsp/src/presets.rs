//! Configurações sugeridas por tipo de microfone e os 4 controles simples.
//!
//! O modo simples tem 4 controles com nomes do dia a dia. Cada um mexe em
//! vários números da cadeia de uma vez, do jeito certo. O modo avançado
//! edita os números direto (`ChainParams`).

use serde::{Deserialize, Serialize};

use crate::calibration::{Recommendation, Warning, TARGET_LUFS};
use crate::chain::ChainParams;
use crate::dynamics::compressor_curve;
use crate::mic::MicType;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SimpleControls {
    /// "Meus amigos me ouvem mais alto ou mais baixo": soma ao ajuste da
    /// calibração (dB, -12 a +12).
    pub volume_db: f32,
    /// "Quanto barulho de fundo tirar": 0 desligada, 1 leve, 2 média, 3 forte.
    pub cleanup: u8,
    /// "Voz mais grave ou mais clara": -3 (encorpada) a +3 (clara).
    pub tone: i8,
    /// "Sussurro e grito no mesmo volume": 0 desligada, 1 leve, 2 média, 3 forte.
    pub evenness: u8,
}

impl Default for SimpleControls {
    fn default() -> Self {
        Self {
            volume_db: 0.0,
            cleanup: 1,
            tone: 0,
            evenness: 2,
        }
    }
}

struct MicBase {
    hpf_hz: f32,
    low: f32,
    presence: f32,
    air: f32,
    deess: bool,
    cleanup: u8,
}

fn base(mic: Option<MicType>) -> MicBase {
    match mic {
        Some(MicType::Dynamic) => MicBase {
            hpf_hz: 80.0,
            low: 0.0,
            presence: 2.5,
            air: 1.5,
            deess: false,
            cleanup: 1,
        },
        Some(MicType::Condenser) => MicBase {
            hpf_hz: 100.0,
            low: -1.0,
            presence: 1.0,
            air: 0.0,
            deess: true,
            cleanup: 2,
        },
        Some(MicType::Headset) => MicBase {
            hpf_hz: 120.0,
            low: -2.0,
            presence: 3.0,
            air: 1.0,
            deess: true,
            cleanup: 2,
        },
        Some(MicType::BuiltIn) => MicBase {
            hpf_hz: 120.0,
            low: -3.0,
            presence: 2.0,
            air: 0.0,
            deess: true,
            cleanup: 3,
        },
        Some(MicType::Lavalier) => MicBase {
            hpf_hz: 100.0,
            low: -1.5,
            presence: 2.0,
            air: 1.0,
            deess: true,
            cleanup: 2,
        },
        None => MicBase {
            hpf_hz: 90.0,
            low: 0.0,
            presence: 1.5,
            air: 0.5,
            deess: false,
            cleanup: 2,
        },
    }
}

/// O que o Sonora sugere para os controles simples.
pub fn suggest_simple(mic: Option<MicType>, rec: Option<&Recommendation>) -> SimpleControls {
    let mut cleanup = base(mic).cleanup;
    let mut evenness = 2;
    if let Some(r) = rec {
        if r.warnings.contains(&Warning::NoisyRoom) {
            cleanup = (cleanup + 1).min(3);
        }
        evenness = match r.loud_peak_after_gain_dbfs {
            Some(p) if p > 0.0 => 3,
            Some(p) if p > -6.0 => 2,
            Some(_) => 1,
            None => 2,
        };
    }
    SimpleControls {
        volume_db: 0.0,
        cleanup,
        tone: 0,
        evenness,
    }
}

/// Monta a cadeia completa a partir dos controles simples.
pub fn build(
    mic: Option<MicType>,
    rec: Option<&Recommendation>,
    s: &SimpleControls,
) -> ChainParams {
    let b = base(mic);
    let calibrated = rec.map(|r| r.gain_db).unwrap_or(0.0);
    let input_gain_db = calibrated + s.volume_db.clamp(-12.0, 12.0);
    let cleanup = s.cleanup.min(3);
    let tone = s.tone.clamp(-3, 3) as f32;

    // Porta de ruído: o limiar da calibração foi medido antes do ganho,
    // e a porta fica depois dele.
    let (gate_known, gate_threshold_db) = match rec {
        Some(r) => match r.gate_threshold_dbfs {
            Some(t) => (true, t + input_gain_db),
            // O próprio microfone já fica mudo nas pausas.
            None => (false, -60.0),
        },
        None => (true, -55.0),
    };

    let (comp_on, comp_threshold_db, comp_ratio) = match s.evenness.min(3) {
        0 => (false, -24.0, 1.0),
        1 => (true, -20.0, 2.0),
        2 => (true, -24.0, 3.0),
        _ => (true, -28.0, 4.5),
    };
    // Recupera parte do volume que o compressor tira da fala normal.
    let typical = TARGET_LUFS + 2.0;
    let comp_makeup_db = if comp_on {
        (-compressor_curve(typical, comp_threshold_db, comp_ratio, 6.0) * 0.8 * 10.0).round() / 10.0
    } else {
        0.0
    };

    let eq_low_db = b.low + if tone < 0.0 { -tone * 1.5 } else { -tone * 0.5 };
    let eq_presence_db = b.presence + tone * 1.2;
    let eq_air_db = b.air + tone.max(0.0);

    ChainParams {
        enabled: true,
        input_gain_db,
        hpf_on: cleanup >= 1,
        hpf_hz: b.hpf_hz + if cleanup >= 3 { 20.0 } else { 0.0 },
        denoise_on: cleanup >= 2,
        gate_on: cleanup >= 1 && gate_known,
        gate_threshold_db: gate_threshold_db + if cleanup >= 3 { 3.0 } else { 0.0 },
        gate_range_db: match cleanup {
            0 | 1 => -15.0,
            2 => -25.0,
            _ => -40.0,
        },
        eq_on: true,
        eq_low_db,
        eq_presence_db,
        eq_air_db,
        deess_on: b.deess || tone >= 2.0,
        comp_on,
        comp_threshold_db,
        comp_ratio,
        comp_makeup_db,
        ..ChainParams::default()
    }
    .sanitized()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mv7_rec() -> Recommendation {
        Recommendation {
            gain_db: 18.8,
            gate_threshold_dbfs: None,
            mic_mutes_itself: true,
            loud_peak_after_gain_dbfs: Some(17.1),
            voice_to_noise_db: None,
            limiter_will_engage: true,
            warnings: vec![],
        }
    }

    #[test]
    fn mv7_case() {
        let rec = mv7_rec();
        let s = suggest_simple(Some(MicType::Dynamic), Some(&rec));
        assert_eq!(s.evenness, 3, "grito passa do teto: compressor forte");
        let p = build(Some(MicType::Dynamic), Some(&rec), &s);
        assert!((p.input_gain_db - 18.8).abs() < 1e-4);
        assert!(!p.gate_on, "MV7 já fica mudo sozinho");
        assert!(p.comp_on && p.comp_ratio > 4.0);
        assert!(p.limiter_on);
        assert!(p.hpf_on);
    }

    #[test]
    fn volume_adds_to_calibration() {
        let rec = mv7_rec();
        let s = SimpleControls {
            volume_db: 3.0,
            ..Default::default()
        };
        assert!((build(None, Some(&rec), &s).input_gain_db - 21.8).abs() < 1e-4);
    }

    #[test]
    fn gate_threshold_moves_with_gain() {
        let rec = Recommendation {
            gate_threshold_dbfs: Some(-60.0),
            gain_db: 10.0,
            mic_mutes_itself: false,
            ..mv7_rec()
        };
        let p = build(
            Some(MicType::Condenser),
            Some(&rec),
            &SimpleControls {
                cleanup: 2,
                ..Default::default()
            },
        );
        assert!(p.gate_on);
        assert!((p.gate_threshold_db + 50.0).abs() < 1e-4);
        assert!(p.denoise_on);
    }

    #[test]
    fn cleanup_zero_turns_cleaning_off() {
        let p = build(
            None,
            None,
            &SimpleControls {
                cleanup: 0,
                ..Default::default()
            },
        );
        assert!(!p.hpf_on && !p.denoise_on && !p.gate_on);
    }

    #[test]
    fn tone_moves_eq() {
        let dark = build(
            None,
            None,
            &SimpleControls {
                tone: -3,
                ..Default::default()
            },
        );
        let bright = build(
            None,
            None,
            &SimpleControls {
                tone: 3,
                ..Default::default()
            },
        );
        assert!(dark.eq_low_db > bright.eq_low_db);
        assert!(bright.eq_presence_db > dark.eq_presence_db);
    }
}
