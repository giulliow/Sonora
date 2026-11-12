//! Medidores de nível: pico e RMS, em dBFS.
//!
//! dBFS = decibéis em relação ao máximo digital. 0 dBFS é o teto: passou
//! disso, o som distorce ("estoura"). Voz bem ajustada fica com picos
//! entre -12 e -3 dBFS.

use crate::db_from_amp;
use serde::Serialize;

/// Pico (maior amostra em valor absoluto) em dBFS.
pub fn peak_dbfs(samples: &[f32]) -> f32 {
    db_from_amp(samples.iter().fold(0.0f32, |m, s| m.max(s.abs())))
}

/// Nível médio de energia (RMS) em dBFS.
pub fn rms_dbfs(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return crate::MIN_DB;
    }
    let sum: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    db_from_amp((sum / samples.len() as f64).sqrt() as f32)
}

/// Em que faixa do medidor o som está. É o que vira cor na tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    /// Abaixo de -24 dBFS de pico: baixo demais para conversa.
    Low,
    /// Faixa boa para voz.
    Good,
    /// Perto do teto: ainda não distorce, mas está no limite.
    Hot,
    /// Bateu no teto: distorceu.
    Clip,
}

pub fn zone_for_peak(peak_dbfs: f32) -> Zone {
    if peak_dbfs >= -0.5 {
        Zone::Clip
    } else if peak_dbfs >= -3.0 {
        Zone::Hot
    } else if peak_dbfs >= -24.0 {
        Zone::Good
    } else {
        Zone::Low
    }
}

/// Medidor de pico com "segura e cai devagar", para a barra não piscar.
#[derive(Debug, Clone)]
pub struct PeakMeter {
    current_db: f32,
    hold_db: f32,
    hold_left: f32,
    /// Quanto o ponteiro cai por segundo (dB/s).
    fall_db_per_sec: f32,
    /// Quanto tempo o pico máximo fica marcado (s).
    hold_secs: f32,
}

impl Default for PeakMeter {
    fn default() -> Self {
        Self {
            current_db: crate::MIN_DB,
            hold_db: crate::MIN_DB,
            hold_left: 0.0,
            fall_db_per_sec: 24.0,
            hold_secs: 1.5,
        }
    }
}

impl PeakMeter {
    /// Atualiza com um bloco de `secs` segundos de áudio.
    pub fn update(&mut self, block: &[f32], secs: f32) {
        let p = peak_dbfs(block);
        let fallen = self.current_db - self.fall_db_per_sec * secs;
        self.current_db = p.max(fallen).max(crate::MIN_DB);
        if p >= self.hold_db {
            self.hold_db = p;
            self.hold_left = self.hold_secs;
        } else {
            self.hold_left -= secs;
            if self.hold_left <= 0.0 {
                self.hold_db = self.current_db;
            }
        }
    }

    pub fn current_db(&self) -> f32 {
        self.current_db
    }

    pub fn hold_db(&self) -> f32 {
        self.hold_db
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::sine;

    #[test]
    fn sine_peak_and_rms() {
        let s = sine(1000.0, 0.5, 1.0, 48_000);
        assert!((peak_dbfs(&s) + 6.02).abs() < 0.05);
        // RMS de uma senoide = pico - 3,01 dB
        assert!((rms_dbfs(&s) + 9.03).abs() < 0.05);
    }

    #[test]
    fn zones() {
        assert_eq!(zone_for_peak(-40.0), Zone::Low);
        assert_eq!(zone_for_peak(-10.0), Zone::Good);
        assert_eq!(zone_for_peak(-2.0), Zone::Hot);
        assert_eq!(zone_for_peak(0.0), Zone::Clip);
    }

    #[test]
    fn meter_falls_slowly() {
        let mut m = PeakMeter::default();
        m.update(&[0.5], 0.05);
        let after_peak = m.current_db();
        m.update(&[0.0], 0.05);
        assert!(m.current_db() < after_peak);
        assert!(m.current_db() > after_peak - 2.0);
        assert!((m.hold_db() - after_peak).abs() < 1e-3);
    }
}
