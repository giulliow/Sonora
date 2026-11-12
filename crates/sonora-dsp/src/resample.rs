//! Troca a taxa de amostragem (ex.: mic em 44,1 kHz, fone em 48 kHz).
//! Interpolação linear: simples e suficiente para ouvir uma gravação de volta.

pub fn resample_linear(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from_hz as f64 / to_hz as f64;
    let out_len = ((input.len() as f64) / ratio).floor() as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos.floor() as usize;
            let frac = (pos - idx as f64) as f32;
            let a = input[idx];
            let b = *input.get(idx + 1).unwrap_or(&a);
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_scales() {
        let v = vec![0.0; 44_100];
        assert_eq!(resample_linear(&v, 44_100, 48_000).len(), 48_000);
    }

    #[test]
    fn identity() {
        let v = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_linear(&v, 48_000, 48_000), v);
    }
}
