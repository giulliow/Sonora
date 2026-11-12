//! Supressão de ruído com IA leve (RNNoise, via `nnnoiseless`).
//!
//! O RNNoise trabalha em blocos de 10 ms a 48 kHz. Guardamos amostras até
//! completar um bloco, então ele acrescenta 10 ms de atraso, imperceptível
//! em conversa.

use nnnoiseless::DenoiseState;
use std::collections::VecDeque;

pub const DENOISE_SAMPLE_RATE: u32 = 48_000;
const FRAME: usize = DenoiseState::FRAME_SIZE;
/// O RNNoise espera amostras na escala de 16 bits.
const SCALE: f32 = 32_768.0;

pub struct Denoiser {
    state: Box<DenoiseState<'static>>,
    input: Vec<f32>,
    output: VecDeque<f32>,
    frame_out: Vec<f32>,
}

impl Default for Denoiser {
    fn default() -> Self {
        Self::new()
    }
}

impl Denoiser {
    pub fn new() -> Self {
        let mut output = VecDeque::with_capacity(FRAME * 3);
        output.extend(std::iter::repeat_n(0.0, FRAME));
        Self {
            state: DenoiseState::new(),
            input: Vec::with_capacity(FRAME),
            output,
            frame_out: vec![0.0; FRAME],
        }
    }

    /// Atraso que este efeito acrescenta, em amostras.
    pub const LATENCY: usize = FRAME;

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.input.push(x * SCALE);
        if self.input.len() == FRAME {
            self.state.process_frame(&mut self.frame_out, &self.input);
            self.input.clear();
            self.output.extend(self.frame_out.iter().map(|s| s / SCALE));
        }
        self.output.pop_front().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amp_from_db;
    use crate::level::rms_dbfs;
    use crate::test_util::noise;

    #[test]
    fn reduces_steady_noise() {
        let mut d = Denoiser::new();
        // Ruído de fundo típico de quarto (~-60 dBFS). O RNNoise leva
        // alguns segundos para "aprender" o ruído, então medimos o fim.
        let n = noise(amp_from_db(-60.0), 8.0, 48_000);
        let out: Vec<f32> = n.iter().map(|x| d.process(*x)).collect();
        assert_eq!(out.len(), n.len());
        let (a, b) = (rms_dbfs(&out[240_000..]), rms_dbfs(&n[240_000..]));
        assert!(a < b - 10.0, "{a} {b}");
    }
}
