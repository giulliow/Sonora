//! Volume percebido (loudness) em LUFS, pelo padrão EBU R128.
//!
//! O pico diz se vai estourar; o LUFS diz se as pessoas acham alto ou baixo.
//! Ele pondera as frequências como o ouvido humano. Rádio e streaming usam
//! esse padrão. Para conversa no Discord, o Sonora mira em -18 LUFS.

use ebur128::{EbuR128, Mode};

fn finite_or_none(v: Result<f64, ebur128::Error>) -> Option<f32> {
    match v {
        Ok(x) if x.is_finite() => Some(x as f32),
        _ => None,
    }
}

/// Medidor contínuo, para a barra ao vivo.
pub struct LoudnessMeter {
    inner: EbuR128,
}

impl LoudnessMeter {
    pub fn new(sample_rate: u32) -> Self {
        let inner = EbuR128::new(1, sample_rate, Mode::M | Mode::S)
            .expect("taxa de amostragem suportada pelo EBU R128");
        Self { inner }
    }

    pub fn push(&mut self, mono: &[f32]) {
        // Só falha com tamanho de buffer inválido para o número de canais,
        // o que não acontece em mono.
        let _ = self.inner.add_frames_f32(mono);
    }

    /// Últimos 400 ms. `None` = silêncio.
    pub fn momentary(&self) -> Option<f32> {
        finite_or_none(self.inner.loudness_momentary())
    }

    /// Últimos 3 s. `None` = silêncio.
    pub fn short_term(&self) -> Option<f32> {
        finite_or_none(self.inner.loudness_shortterm())
    }
}

/// Loudness integrado de um trecho inteiro, ignorando as pausas
/// (o "gate" do padrão descarta trechos muito mais baixos que a fala).
pub fn integrated_lufs(mono: &[f32], sample_rate: u32) -> Option<f32> {
    let mut m = EbuR128::new(1, sample_rate, Mode::I).ok()?;
    m.add_frames_f32(mono).ok()?;
    finite_or_none(m.loudness_global())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::sine;

    #[test]
    fn full_scale_1k_sine_is_about_minus_3() {
        // Referência do padrão: senoide de 1 kHz a 0 dBFS ≈ -3,01 LUFS.
        let s = sine(1000.0, 1.0, 3.0, 48_000);
        let l = integrated_lufs(&s, 48_000).unwrap();
        assert!((l + 3.01).abs() < 0.1, "{l}");
    }

    #[test]
    fn silence_is_none() {
        assert_eq!(integrated_lufs(&vec![0.0; 48_000], 48_000), None);
    }

    #[test]
    fn live_meter_reports() {
        let mut m = LoudnessMeter::new(48_000);
        m.push(&sine(1000.0, 0.1, 1.0, 48_000));
        let v = m.momentary().unwrap();
        assert!((v + 23.0).abs() < 0.3, "{v}");
    }
}
