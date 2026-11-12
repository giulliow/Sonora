//! A cadeia de som completa, na ordem certa:
//! ganho → filtro de graves → supressão de ruído → porta de ruído →
//! equalizador → de-esser → compressor → limitador.
//!
//! Primeiro se limpa, depois se deixa bonito, por último se protege
//! contra estouros.

use serde::{Deserialize, Serialize};

use crate::amp_from_db;
use crate::biquad::Biquad;
use crate::denoise::{Denoiser, DENOISE_SAMPLE_RATE};
use crate::dynamics::{
    Compressor, CompressorSettings, DeEsser, DeEsserSettings, Gate, GateSettings,
};
use crate::process::Limiter;

/// Todos os números da cadeia. É isso que a tela edita, o app salva
/// e o módulo dentro do Windows lê.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChainParams {
    /// Liga/desliga tudo. Desligado = som passa intocado.
    pub enabled: bool,

    pub input_gain_db: f32,

    pub hpf_on: bool,
    pub hpf_hz: f32,

    pub denoise_on: bool,

    pub gate_on: bool,
    pub gate_threshold_db: f32,
    pub gate_range_db: f32,
    pub gate_hold_ms: f32,
    pub gate_release_ms: f32,

    pub eq_on: bool,
    /// Corpo (grave), prateleira em 150 Hz.
    pub eq_low_db: f32,
    /// Clareza (presença), sino em 3,5 kHz.
    pub eq_presence_db: f32,
    /// Brilho (ar), prateleira em 10 kHz.
    pub eq_air_db: f32,

    pub deess_on: bool,
    pub deess_threshold_db: f32,
    pub deess_max_reduction_db: f32,

    pub comp_on: bool,
    pub comp_threshold_db: f32,
    pub comp_ratio: f32,
    pub comp_attack_ms: f32,
    pub comp_release_ms: f32,
    pub comp_makeup_db: f32,

    pub limiter_on: bool,
    pub limiter_ceiling_db: f32,
}

impl Default for ChainParams {
    /// Neutro: tudo desligado menos o limitador.
    fn default() -> Self {
        Self {
            enabled: true,
            input_gain_db: 0.0,
            hpf_on: false,
            hpf_hz: 80.0,
            denoise_on: false,
            gate_on: false,
            gate_threshold_db: -50.0,
            gate_range_db: -30.0,
            gate_hold_ms: 150.0,
            gate_release_ms: 120.0,
            eq_on: false,
            eq_low_db: 0.0,
            eq_presence_db: 0.0,
            eq_air_db: 0.0,
            deess_on: false,
            deess_threshold_db: -28.0,
            deess_max_reduction_db: 6.0,
            comp_on: false,
            comp_threshold_db: -24.0,
            comp_ratio: 3.0,
            comp_attack_ms: 5.0,
            comp_release_ms: 120.0,
            comp_makeup_db: 0.0,
            limiter_on: true,
            limiter_ceiling_db: -1.0,
        }
    }
}

/// Uma entrada da tabela de parâmetros: nome estável (usado na
/// configuração do Windows), como ler e como escrever.
pub struct ParamSpec {
    pub name: &'static str,
    pub get: fn(&ChainParams) -> f32,
    pub set: fn(&mut ChainParams, f32),
}

macro_rules! num {
    ($name:literal, $field:ident) => {
        ParamSpec {
            name: $name,
            get: |p| p.$field,
            set: |p, v| p.$field = v,
        }
    };
}
macro_rules! flag {
    ($name:literal, $field:ident) => {
        ParamSpec {
            name: $name,
            get: |p| if p.$field { 1.0 } else { 0.0 },
            set: |p, v| p.$field = v >= 0.5,
        }
    };
}

/// Ordem fixa: o módulo VST expõe os parâmetros nesta ordem.
pub const PARAMS: &[ParamSpec] = &[
    flag!("enabled", enabled),
    num!("input_gain_db", input_gain_db),
    flag!("hpf_on", hpf_on),
    num!("hpf_hz", hpf_hz),
    flag!("denoise_on", denoise_on),
    flag!("gate_on", gate_on),
    num!("gate_threshold_db", gate_threshold_db),
    num!("gate_range_db", gate_range_db),
    num!("gate_hold_ms", gate_hold_ms),
    num!("gate_release_ms", gate_release_ms),
    flag!("eq_on", eq_on),
    num!("eq_low_db", eq_low_db),
    num!("eq_presence_db", eq_presence_db),
    num!("eq_air_db", eq_air_db),
    flag!("deess_on", deess_on),
    num!("deess_threshold_db", deess_threshold_db),
    num!("deess_max_reduction_db", deess_max_reduction_db),
    flag!("comp_on", comp_on),
    num!("comp_threshold_db", comp_threshold_db),
    num!("comp_ratio", comp_ratio),
    num!("comp_attack_ms", comp_attack_ms),
    num!("comp_release_ms", comp_release_ms),
    num!("comp_makeup_db", comp_makeup_db),
    flag!("limiter_on", limiter_on),
    num!("limiter_ceiling_db", limiter_ceiling_db),
];

impl ChainParams {
    /// Garante que nenhum número absurdo chegue no processamento.
    pub fn sanitized(mut self) -> Self {
        fn c(v: f32, lo: f32, hi: f32, def: f32) -> f32 {
            if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                def
            }
        }
        let d = ChainParams::default();
        self.input_gain_db = c(self.input_gain_db, -30.0, 40.0, 0.0);
        self.hpf_hz = c(self.hpf_hz, 20.0, 300.0, d.hpf_hz);
        self.gate_threshold_db = c(self.gate_threshold_db, -100.0, 0.0, d.gate_threshold_db);
        self.gate_range_db = c(self.gate_range_db, -90.0, 0.0, d.gate_range_db);
        self.gate_hold_ms = c(self.gate_hold_ms, 0.0, 2000.0, d.gate_hold_ms);
        self.gate_release_ms = c(self.gate_release_ms, 5.0, 2000.0, d.gate_release_ms);
        self.eq_low_db = c(self.eq_low_db, -12.0, 12.0, 0.0);
        self.eq_presence_db = c(self.eq_presence_db, -12.0, 12.0, 0.0);
        self.eq_air_db = c(self.eq_air_db, -12.0, 12.0, 0.0);
        self.deess_threshold_db = c(self.deess_threshold_db, -60.0, 0.0, d.deess_threshold_db);
        self.deess_max_reduction_db = c(
            self.deess_max_reduction_db,
            0.0,
            24.0,
            d.deess_max_reduction_db,
        );
        self.comp_threshold_db = c(self.comp_threshold_db, -60.0, 0.0, d.comp_threshold_db);
        self.comp_ratio = c(self.comp_ratio, 1.0, 20.0, d.comp_ratio);
        self.comp_attack_ms = c(self.comp_attack_ms, 0.1, 200.0, d.comp_attack_ms);
        self.comp_release_ms = c(self.comp_release_ms, 10.0, 2000.0, d.comp_release_ms);
        self.comp_makeup_db = c(self.comp_makeup_db, -12.0, 24.0, 0.0);
        self.limiter_ceiling_db = c(self.limiter_ceiling_db, -12.0, 0.0, d.limiter_ceiling_db);
        self
    }
}

/// O processador. Um por canal.
pub struct VoiceChain {
    p: ChainParams,
    sr: f32,
    gain: f32,
    hpf: Biquad,
    denoiser: Option<Denoiser>,
    gate: Gate,
    eq: [Biquad; 3],
    deesser: DeEsser,
    comp: Compressor,
    limiter: Limiter,
}

fn gate_settings(p: &ChainParams) -> GateSettings {
    GateSettings {
        threshold_db: p.gate_threshold_db,
        range_db: p.gate_range_db,
        hold_ms: p.gate_hold_ms,
        release_ms: p.gate_release_ms,
    }
}

fn comp_settings(p: &ChainParams) -> CompressorSettings {
    CompressorSettings {
        threshold_db: p.comp_threshold_db,
        ratio: p.comp_ratio,
        attack_ms: p.comp_attack_ms,
        release_ms: p.comp_release_ms,
        knee_db: 6.0,
        makeup_db: p.comp_makeup_db,
    }
}

fn deess_settings(p: &ChainParams) -> DeEsserSettings {
    DeEsserSettings {
        threshold_db: p.deess_threshold_db,
        max_reduction_db: p.deess_max_reduction_db,
    }
}

fn eq_filters(p: &ChainParams, sr: f32) -> [Biquad; 3] {
    [
        Biquad::low_shelf(150.0, p.eq_low_db, sr),
        Biquad::peaking(3500.0, 0.9, p.eq_presence_db, sr),
        Biquad::high_shelf(10_000.0, p.eq_air_db, sr),
    ]
}

impl VoiceChain {
    pub fn new(params: ChainParams, sample_rate: u32) -> Self {
        let p = params.sanitized();
        let sr = sample_rate as f32;
        Self {
            p,
            sr,
            gain: amp_from_db(p.input_gain_db),
            hpf: Biquad::highpass(p.hpf_hz, 0.707, sr),
            // O RNNoise só funciona a 48 kHz; em outra taxa, fica desligado.
            denoiser: (sample_rate == DENOISE_SAMPLE_RATE).then(Denoiser::new),
            gate: Gate::new(gate_settings(&p), sr),
            eq: eq_filters(&p, sr),
            deesser: DeEsser::new(deess_settings(&p), sr),
            comp: Compressor::new(comp_settings(&p), sr),
            limiter: Limiter::new(p.limiter_ceiling_db, 80.0, sample_rate),
        }
    }

    /// Supressão de ruído disponível nesta taxa de amostragem?
    pub fn denoise_available(&self) -> bool {
        self.denoiser.is_some()
    }

    pub fn params(&self) -> &ChainParams {
        &self.p
    }

    /// Troca os parâmetros sem zerar os filtros (sem estalos).
    pub fn set_params(&mut self, params: ChainParams) {
        let p = params.sanitized();
        self.gain = amp_from_db(p.input_gain_db);
        self.hpf.retune(Biquad::highpass(p.hpf_hz, 0.707, self.sr));
        self.gate.set(gate_settings(&p));
        let eq = eq_filters(&p, self.sr);
        for (f, n) in self.eq.iter_mut().zip(eq) {
            f.retune(n);
        }
        self.deesser.set(deess_settings(&p));
        self.comp.set(comp_settings(&p));
        if p.limiter_ceiling_db != self.p.limiter_ceiling_db {
            self.limiter = Limiter::new(p.limiter_ceiling_db, 80.0, self.sr as u32);
        }
        self.p = p;
    }

    #[inline]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let p = &self.p;
        if !p.enabled {
            return x;
        }
        let mut s = x * self.gain;
        if p.hpf_on {
            s = self.hpf.process(s);
        }
        if p.denoise_on {
            if let Some(d) = self.denoiser.as_mut() {
                s = d.process(s);
            }
        }
        if p.gate_on {
            s = self.gate.process(s);
        }
        if p.eq_on {
            for f in self.eq.iter_mut() {
                s = f.process(s);
            }
        }
        if p.deess_on {
            s = self.deesser.process(s);
        }
        if p.comp_on {
            s = self.comp.process(s);
        }
        if p.limiter_on {
            s = self.limiter.process_sample(s);
        }
        s
    }

    pub fn process(&mut self, buf: &mut [f32]) {
        for s in buf {
            *s = self.process_sample(*s);
        }
    }
}

/// Processa um trecho gravado inteiro (para "ouvir com o ajuste").
pub fn render(samples: &[f32], params: ChainParams, sample_rate: u32) -> Vec<f32> {
    let mut chain = VoiceChain::new(params, sample_rate);
    let mut out = samples.to_vec();
    chain.process(&mut out);
    // Compensa o atraso da supressão de ruído para a prévia começar no mesmo ponto.
    if params.enabled && params.denoise_on && chain.denoise_available() {
        out.drain(..Denoiser::LATENCY.min(out.len()));
        out.extend(std::iter::repeat_n(
            0.0,
            Denoiser::LATENCY.min(samples.len()),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::peak_dbfs;
    use crate::test_util::sine;

    #[test]
    fn param_table_roundtrip() {
        let mut p = ChainParams::default();
        for (i, spec) in PARAMS.iter().enumerate() {
            let v = if spec.name.ends_with("_on") || spec.name == "enabled" {
                1.0
            } else {
                i as f32 + 0.5
            };
            (spec.set)(&mut p, v);
            assert_eq!((spec.get)(&p), v, "{}", spec.name);
        }
        // nomes únicos
        let mut names: Vec<_> = PARAMS.iter().map(|s| s.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), PARAMS.len());
    }

    #[test]
    fn disabled_is_bit_exact_passthrough() {
        let x = sine(440.0, 0.5, 0.2, 48_000);
        let p = ChainParams {
            enabled: false,
            input_gain_db: 20.0,
            ..Default::default()
        };
        assert_eq!(render(&x, p, 48_000), x);
    }

    #[test]
    fn full_chain_never_exceeds_ceiling() {
        let x = sine(440.0, 0.9, 1.0, 48_000);
        let p = ChainParams {
            input_gain_db: 20.0,
            hpf_on: true,
            denoise_on: true,
            gate_on: true,
            eq_on: true,
            eq_presence_db: 6.0,
            deess_on: true,
            comp_on: true,
            ..Default::default()
        };
        let y = render(&x, p, 48_000);
        assert_eq!(y.len(), x.len());
        assert!(peak_dbfs(&y) <= -1.0 + 1e-3);
    }

    #[test]
    fn sanitizes_garbage() {
        let p = ChainParams {
            input_gain_db: f32::NAN,
            comp_ratio: 0.0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(p.input_gain_db, 0.0);
        assert_eq!(p.comp_ratio, 1.0);
    }

    #[test]
    fn works_at_44100_without_denoise() {
        let chain = VoiceChain::new(
            ChainParams {
                denoise_on: true,
                ..Default::default()
            },
            44_100,
        );
        assert!(!chain.denoise_available());
    }
}
