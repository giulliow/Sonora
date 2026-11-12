//! Processamento aplicado na voz. Na fase 1: ganho e limitador,
//! usados para tocar a gravação "com o ajuste sugerido".

use crate::amp_from_db;

/// Limitador: uma "parede" que nunca deixa o som passar do teto.
/// Ataque instantâneo (não deixa passar nem uma amostra acima do teto)
/// e soltura suave, para não "bombear".
#[derive(Debug, Clone)]
pub struct Limiter {
    ceiling: f32,
    gain: f32,
    release_coef: f32,
}

impl Limiter {
    /// `ceiling_db`: teto em dBFS (ex.: -1). `release_ms`: tempo de soltura.
    pub fn new(ceiling_db: f32, release_ms: f32, sample_rate: u32) -> Self {
        let release_samples = (release_ms / 1000.0 * sample_rate as f32).max(1.0);
        Self {
            ceiling: amp_from_db(ceiling_db),
            gain: 1.0,
            release_coef: 1.0 - (-1.0 / release_samples).exp(),
        }
    }

    #[inline]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let target = if a > self.ceiling {
            self.ceiling / a
        } else {
            1.0
        };
        if target < self.gain {
            self.gain = target;
        } else {
            self.gain += (target - self.gain) * self.release_coef;
        }
        x * self.gain
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        for s in buf {
            *s = self.process_sample(*s);
        }
    }
}

/// Aplica ganho e limitador num trecho já gravado.
pub fn apply_gain_and_limit(samples: &[f32], gain_db: f32, sample_rate: u32) -> Vec<f32> {
    let g = amp_from_db(gain_db);
    let mut lim = Limiter::new(-1.0, 80.0, sample_rate);
    samples.iter().map(|s| lim.process_sample(s * g)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::peak_dbfs;
    use crate::test_util::sine;

    #[test]
    fn limiter_never_exceeds_ceiling() {
        let s = sine(200.0, 0.9, 1.0, 48_000);
        let out = apply_gain_and_limit(&s, 12.0, 48_000);
        assert!(peak_dbfs(&out) <= -1.0 + 1e-3, "{}", peak_dbfs(&out));
    }

    #[test]
    fn quiet_signal_only_gets_gain() {
        let s = sine(200.0, 0.01, 1.0, 48_000);
        let out = apply_gain_and_limit(&s, 6.0, 48_000);
        let expected = peak_dbfs(&s) + 6.0;
        assert!((peak_dbfs(&out) - expected).abs() < 0.05);
    }
}
