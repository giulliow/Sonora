//! Efeitos que mexem no volume conforme o som: porta de ruído, compressor e de-esser.

use crate::biquad::Biquad;
use crate::{amp_from_db, db_from_amp};

/// Coeficiente de suavização para um tempo em milissegundos.
fn coef(ms: f32, sr: f32) -> f32 {
    let n = (ms.max(0.01) / 1000.0 * sr).max(1.0);
    (-1.0 / n).exp()
}

/// Seguidor de envelope: acompanha o "tamanho" do som ao longo do tempo.
#[derive(Debug, Clone, Copy)]
struct Envelope {
    value: f32,
    attack: f32,
    release: f32,
}

impl Envelope {
    fn new(attack_ms: f32, release_ms: f32, sr: f32) -> Self {
        Self {
            value: 0.0,
            attack: coef(attack_ms, sr),
            release: coef(release_ms, sr),
        }
    }

    #[inline]
    fn next(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let c = if a > self.value {
            self.attack
        } else {
            self.release
        };
        self.value = c * self.value + (1.0 - c) * a;
        self.value
    }
}

// ---------------------------------------------------------------- porta de ruído

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateSettings {
    /// Abre quando o som passa deste nível (dBFS).
    pub threshold_db: f32,
    /// Quanto abaixa quando fechada (dB, negativo). -80 = quase mudo.
    pub range_db: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
}

/// Porta de ruído: abaixa o microfone quando você não está falando.
/// Fecha 4 dB abaixo de onde abre, para não ficar "piscando".
#[derive(Debug, Clone)]
pub struct Gate {
    s: GateSettings,
    sr: f32,
    env: Envelope,
    open: bool,
    hold_left: f32,
    gain: f32,
    open_coef: f32,
    close_coef: f32,
}

const GATE_HYSTERESIS_DB: f32 = 4.0;

impl Gate {
    pub fn new(s: GateSettings, sr: f32) -> Self {
        let mut g = Self {
            s,
            sr,
            env: Envelope::new(0.5, 40.0, sr),
            open: false,
            hold_left: 0.0,
            gain: amp_from_db(s.range_db),
            open_coef: coef(1.0, sr),
            close_coef: coef(s.release_ms, sr),
        };
        g.set(s);
        g
    }

    pub fn set(&mut self, s: GateSettings) {
        self.s = s;
        self.close_coef = coef(s.release_ms, self.sr);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let level = db_from_amp(self.env.next(x));
        if level >= self.s.threshold_db {
            self.open = true;
            self.hold_left = self.s.hold_ms / 1000.0 * self.sr;
        } else if self.open && level < self.s.threshold_db - GATE_HYSTERESIS_DB {
            if self.hold_left > 0.0 {
                self.hold_left -= 1.0;
            } else {
                self.open = false;
            }
        }
        let target = if self.open {
            1.0
        } else {
            amp_from_db(self.s.range_db)
        };
        let c = if target > self.gain {
            self.open_coef
        } else {
            self.close_coef
        };
        self.gain = c * self.gain + (1.0 - c) * target;
        x * self.gain
    }

    pub fn is_open(&self) -> bool {
        self.open
    }
}

// ---------------------------------------------------------------- compressor

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressorSettings {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    /// Largura da "curva suave" em volta do limiar (dB).
    pub knee_db: f32,
    /// Ganho depois da compressão, para recuperar o volume (dB).
    pub makeup_db: f32,
}

/// Quanto o compressor reduz (dB, ≤ 0) para um nível de entrada.
pub fn compressor_curve(level_db: f32, threshold_db: f32, ratio: f32, knee_db: f32) -> f32 {
    let over = level_db - threshold_db;
    let slope = 1.0 / ratio.max(1.0) - 1.0;
    if knee_db > 0.0 && over.abs() <= knee_db / 2.0 {
        slope * (over + knee_db / 2.0).powi(2) / (2.0 * knee_db)
    } else if over > 0.0 {
        slope * over
    } else {
        0.0
    }
}

/// Compressor: sobe as partes baixas e segura as altas, deixando a voz uniforme.
#[derive(Debug, Clone)]
pub struct Compressor {
    s: CompressorSettings,
    sr: f32,
    env: Envelope,
    makeup: f32,
    last_reduction_db: f32,
}

impl Compressor {
    pub fn new(s: CompressorSettings, sr: f32) -> Self {
        Self {
            s,
            sr,
            env: Envelope::new(s.attack_ms, s.release_ms, sr),
            makeup: amp_from_db(s.makeup_db),
            last_reduction_db: 0.0,
        }
    }

    pub fn set(&mut self, s: CompressorSettings) {
        let value = self.env.value;
        self.env = Envelope::new(s.attack_ms, s.release_ms, self.sr);
        self.env.value = value;
        self.makeup = amp_from_db(s.makeup_db);
        self.s = s;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let level = db_from_amp(self.env.next(x));
        let red = compressor_curve(level, self.s.threshold_db, self.s.ratio, self.s.knee_db);
        self.last_reduction_db = red;
        x * amp_from_db(red) * self.makeup
    }

    pub fn reduction_db(&self) -> f32 {
        self.last_reduction_db
    }
}

// ---------------------------------------------------------------- de-esser

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeEsserSettings {
    /// A partir de que nível os agudos começam a ser abaixados (dBFS).
    pub threshold_db: f32,
    /// Máximo que abaixa os "S" (dB, positivo).
    pub max_reduction_db: f32,
}

/// De-esser: separa os agudos (acima de ~5 kHz) e abaixa só eles quando
/// ficam fortes demais, como nos "S" e "X".
#[derive(Debug, Clone)]
pub struct DeEsser {
    s: DeEsserSettings,
    split: Biquad,
    env: Envelope,
}

const DEESS_SPLIT_HZ: f32 = 5000.0;

impl DeEsser {
    pub fn new(s: DeEsserSettings, sr: f32) -> Self {
        Self {
            s,
            split: Biquad::lowpass(DEESS_SPLIT_HZ, 0.707, sr),
            env: Envelope::new(1.0, 60.0, sr),
        }
    }

    pub fn set(&mut self, s: DeEsserSettings) {
        self.s = s;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let low = self.split.process(x);
        let high = x - low; // low + high == x: sem mudança quando não há redução
        let level = db_from_amp(self.env.next(high));
        let red =
            compressor_curve(level, self.s.threshold_db, 4.0, 6.0).max(-self.s.max_reduction_db);
        low + high * amp_from_db(red)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{peak_dbfs, rms_dbfs};
    use crate::test_util::{noise, sine};

    const SR: f32 = 48_000.0;

    #[test]
    fn curve_shape() {
        assert_eq!(compressor_curve(-40.0, -20.0, 4.0, 0.0), 0.0);
        assert!((compressor_curve(-8.0, -20.0, 4.0, 0.0) + 9.0).abs() < 1e-4);
        // dentro do joelho reduz um pouco, menos que fora
        let k = compressor_curve(-20.0, -20.0, 4.0, 6.0);
        assert!(k < 0.0 && k > -2.0);
    }

    #[test]
    fn compressor_evens_out_levels() {
        let s = CompressorSettings {
            threshold_db: -24.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 100.0,
            knee_db: 6.0,
            makeup_db: 0.0,
        };
        let run = |amp_db: f32| {
            let mut c = Compressor::new(s, SR);
            let x = sine(300.0, amp_from_db(amp_db), 1.0, 48_000);
            let y: Vec<f32> = x.iter().map(|v| c.process(*v)).collect();
            rms_dbfs(&y[24_000..])
        };
        let quiet = run(-30.0);
        let loud = run(-6.0);
        // entrada com 24 dB de diferença sai com bem menos
        assert!(loud - quiet < 12.0, "{quiet} {loud}");
    }

    #[test]
    fn gate_mutes_noise_and_passes_voice() {
        let gs = GateSettings {
            threshold_db: -45.0,
            range_db: -40.0,
            hold_ms: 100.0,
            release_ms: 80.0,
        };
        let mut g = Gate::new(gs, SR);
        let n = noise(amp_from_db(-60.0), 1.0, 48_000);
        let out: Vec<f32> = n.iter().map(|v| g.process(*v)).collect();
        assert!(rms_dbfs(&out[24_000..]) < rms_dbfs(&n[24_000..]) - 30.0);

        let v = sine(300.0, amp_from_db(-20.0), 1.0, 48_000);
        let out: Vec<f32> = v.iter().map(|x| g.process(*x)).collect();
        assert!((peak_dbfs(&out[24_000..]) - peak_dbfs(&v[24_000..])).abs() < 0.5);
        assert!(g.is_open());
    }

    #[test]
    fn deesser_only_touches_highs() {
        let ds = DeEsserSettings {
            threshold_db: -30.0,
            max_reduction_db: 8.0,
        };
        let mut d = DeEsser::new(ds, SR);
        let lowtone = sine(300.0, 0.3, 1.0, 48_000);
        let out: Vec<f32> = lowtone.iter().map(|x| d.process(*x)).collect();
        assert!((rms_dbfs(&out[24_000..]) - rms_dbfs(&lowtone[24_000..])).abs() < 0.3);

        let mut d = DeEsser::new(ds, SR);
        let ess = sine(8000.0, 0.3, 1.0, 48_000);
        let out: Vec<f32> = ess.iter().map(|x| d.process(*x)).collect();
        assert!(rms_dbfs(&out[24_000..]) < rms_dbfs(&ess[24_000..]) - 3.0);
    }
}
