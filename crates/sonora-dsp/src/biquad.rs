//! Filtros biquad (receitas clássicas do "Audio EQ Cookbook", de R. Bristow-Johnson).
//! São a base do filtro de graves, do equalizador e do de-esser.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, Default)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// Filtro que não faz nada (deixa tudo passar).
    pub fn bypass() -> Self {
        Self {
            b0: 1.0,
            ..Default::default()
        }
    }

    fn from_raw(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn w0(freq: f32, sr: f32) -> (f32, f32) {
        let w0 = 2.0 * PI * (freq.clamp(10.0, sr * 0.45)) / sr;
        (w0.cos(), w0.sin())
    }

    pub fn highpass(freq: f32, q: f32, sr: f32) -> Self {
        let (c, s) = Self::w0(freq, sr);
        let alpha = s / (2.0 * q);
        Self::from_raw(
            (1.0 + c) / 2.0,
            -(1.0 + c),
            (1.0 + c) / 2.0,
            1.0 + alpha,
            -2.0 * c,
            1.0 - alpha,
        )
    }

    pub fn lowpass(freq: f32, q: f32, sr: f32) -> Self {
        let (c, s) = Self::w0(freq, sr);
        let alpha = s / (2.0 * q);
        Self::from_raw(
            (1.0 - c) / 2.0,
            1.0 - c,
            (1.0 - c) / 2.0,
            1.0 + alpha,
            -2.0 * c,
            1.0 - alpha,
        )
    }

    pub fn peaking(freq: f32, q: f32, gain_db: f32, sr: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let (c, s) = Self::w0(freq, sr);
        let alpha = s / (2.0 * q);
        Self::from_raw(
            1.0 + alpha * a,
            -2.0 * c,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * c,
            1.0 - alpha / a,
        )
    }

    fn shelf(freq: f32, gain_db: f32, sr: f32, high: bool) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let (c, s) = Self::w0(freq, sr);
        // inclinação S = 1
        let alpha = s / 2.0 * 2f32.sqrt();
        let sa = 2.0 * a.sqrt() * alpha;
        if high {
            Self::from_raw(
                a * ((a + 1.0) + (a - 1.0) * c + sa),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
                a * ((a + 1.0) + (a - 1.0) * c - sa),
                (a + 1.0) - (a - 1.0) * c + sa,
                2.0 * ((a - 1.0) - (a + 1.0) * c),
                (a + 1.0) - (a - 1.0) * c - sa,
            )
        } else {
            Self::from_raw(
                a * ((a + 1.0) - (a - 1.0) * c + sa),
                2.0 * a * ((a - 1.0) - (a + 1.0) * c),
                a * ((a + 1.0) - (a - 1.0) * c - sa),
                (a + 1.0) + (a - 1.0) * c + sa,
                -2.0 * ((a - 1.0) + (a + 1.0) * c),
                (a + 1.0) + (a - 1.0) * c - sa,
            )
        }
    }

    pub fn low_shelf(freq: f32, gain_db: f32, sr: f32) -> Self {
        Self::shelf(freq, gain_db, sr, false)
    }

    pub fn high_shelf(freq: f32, gain_db: f32, sr: f32) -> Self {
        Self::shelf(freq, gain_db, sr, true)
    }

    /// Troca os coeficientes mantendo o estado (sem estalo).
    pub fn retune(&mut self, other: Biquad) {
        let (z1, z2) = (self.z1, self.z2);
        *self = other;
        self.z1 = z1;
        self.z2 = z2;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        // Forma direta transposta II
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::rms_dbfs;
    use crate::test_util::sine;

    fn gain_at(mut f: Biquad, freq: f32) -> f32 {
        let s = sine(freq, 0.5, 1.0, 48_000);
        let out: Vec<f32> = s.iter().map(|x| f.process(*x)).collect();
        rms_dbfs(&out[24_000..]) - rms_dbfs(&s[24_000..])
    }

    #[test]
    fn highpass_cuts_lows_keeps_voice() {
        let hp = Biquad::highpass(100.0, 0.707, 48_000.0);
        assert!(gain_at(hp, 30.0) < -15.0);
        assert!(gain_at(hp, 1000.0).abs() < 0.2);
    }

    #[test]
    fn peaking_boosts_center() {
        let p = Biquad::peaking(3500.0, 1.0, 6.0, 48_000.0);
        assert!((gain_at(p, 3500.0) - 6.0).abs() < 0.3);
        assert!(gain_at(p, 200.0).abs() < 0.5);
    }

    #[test]
    fn shelves() {
        let lo = Biquad::low_shelf(150.0, -6.0, 48_000.0);
        assert!((gain_at(lo, 40.0) + 6.0).abs() < 0.6);
        assert!(gain_at(lo, 4000.0).abs() < 0.3);
        let hi = Biquad::high_shelf(10_000.0, 4.0, 48_000.0);
        assert!((gain_at(hi, 18_000.0) - 4.0).abs() < 0.8);
        assert!(gain_at(hi, 500.0).abs() < 0.3);
    }
}
