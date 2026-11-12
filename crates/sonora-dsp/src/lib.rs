//! Núcleo de áudio do Sonora: medição, calibração e processamento de voz.
//!
//! Este crate não fala com placa de som nenhuma. Ele só recebe amostras
//! (`f32`, mono, entre -1.0 e 1.0) e devolve números e sugestões. Assim dá
//! para testar tudo sem microfone.

pub mod biquad;
pub mod calibration;
pub mod chain;
pub mod denoise;
pub mod dynamics;
pub mod level;
pub mod loudness;
pub mod mic;
pub mod presets;
pub mod process;
pub mod resample;

/// Menor valor em dB que o app mostra. Abaixo disso é "silêncio total".
pub const MIN_DB: f32 = -100.0;

/// Converte amplitude linear (0.0 a 1.0) em dBFS.
pub fn db_from_amp(amp: f32) -> f32 {
    if amp <= 1e-5 {
        MIN_DB
    } else {
        (20.0 * amp.log10()).max(MIN_DB)
    }
}

/// Converte dB em amplitude linear.
pub fn amp_from_db(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Junta canais intercalados (L R L R ...) em um canal só, pela média.
pub fn downmix_into(interleaved: &[f32], channels: usize, out: &mut Vec<f32>) {
    if channels <= 1 {
        out.extend_from_slice(interleaved);
        return;
    }
    let inv = 1.0 / channels as f32;
    out.extend(
        interleaved
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() * inv),
    );
}

#[cfg(test)]
pub(crate) mod test_util {
    use std::f32::consts::TAU;

    /// Senoide de `freq` Hz com pico `amp`, `secs` segundos.
    pub fn sine(freq: f32, amp: f32, secs: f32, sr: u32) -> Vec<f32> {
        let n = (secs * sr as f32) as usize;
        (0..n)
            .map(|i| amp * (TAU * freq * i as f32 / sr as f32).sin())
            .collect()
    }

    /// Ruído pseudoaleatório determinístico com pico ~`amp`.
    pub fn noise(amp: f32, secs: f32, sr: u32) -> Vec<f32> {
        let n = (secs * sr as f32) as usize;
        let mut state: u32 = 0x1234_5678;
        (0..n)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                amp * ((state as f32 / u32::MAX as f32) * 2.0 - 1.0)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_roundtrip() {
        assert!((db_from_amp(1.0) - 0.0).abs() < 1e-4);
        assert!((db_from_amp(0.5) + 6.0206).abs() < 1e-3);
        assert!((amp_from_db(-6.0206) - 0.5).abs() < 1e-4);
        assert_eq!(db_from_amp(0.0), MIN_DB);
    }

    #[test]
    fn downmix_stereo() {
        let mut out = Vec::new();
        downmix_into(&[1.0, 0.0, 0.5, 0.5], 2, &mut out);
        assert_eq!(out, vec![0.5, 0.5]);
    }
}
