#![allow(deprecated)]
// o crate `vst` está "deprecated" por causa da licença do VST2 da Steinberg; a ABI segue estável.
//! Módulo VST2 do Sonora.
//!
//! O Equalizer APO carrega este módulo dentro do sistema de áudio do
//! Windows, preso ao microfone. Assim o tratamento vale para todos os
//! programas, sem nenhum app do Sonora rodando. O app só escreve os
//! números na configuração do Equalizer APO, que os entrega aqui como
//! parâmetros com nome (ver `sonora_dsp::chain::PARAMS`).
//!
//! Mono (1 entrada, 1 saída): para um microfone estéreo, o Equalizer APO
//! cria uma instância por canal.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sonora_dsp::chain::{ChainParams, VoiceChain, PARAMS};
use vst::plugin_main;
use vst::prelude::*;

pub struct SharedParams {
    values: Vec<AtomicFloat>,
    dirty: AtomicBool,
}

impl SharedParams {
    fn new() -> Self {
        let d = ChainParams::default();
        Self {
            values: PARAMS
                .iter()
                .map(|p| AtomicFloat::new((p.get)(&d)))
                .collect(),
            dirty: AtomicBool::new(true),
        }
    }

    fn snapshot(&self) -> ChainParams {
        let mut p = ChainParams::default();
        for (spec, v) in PARAMS.iter().zip(&self.values) {
            (spec.set)(&mut p, v.get());
        }
        p
    }
}

impl PluginParameters for SharedParams {
    fn get_parameter_name(&self, index: i32) -> String {
        PARAMS
            .get(index as usize)
            .map(|p| p.name.to_string())
            .unwrap_or_default()
    }

    fn get_parameter(&self, index: i32) -> f32 {
        self.values
            .get(index as usize)
            .map(|v| v.get())
            .unwrap_or(0.0)
    }

    // Recebe o valor já na unidade real (dB, Hz, ms): o Equalizer APO
    // repassa o número exatamente como está na configuração.
    fn set_parameter(&self, index: i32, value: f32) {
        if let Some(v) = self.values.get(index as usize) {
            v.set(value);
            self.dirty.store(true, Ordering::Release);
        }
    }

    fn get_parameter_text(&self, index: i32) -> String {
        format!("{:.2}", self.get_parameter(index))
    }
}

pub struct SonoraVst {
    params: Arc<SharedParams>,
    sample_rate: u32,
    chain: VoiceChain,
}

impl SonoraVst {
    fn rebuild(&mut self) {
        self.chain = VoiceChain::new(self.params.snapshot(), self.sample_rate);
        self.params.dirty.store(false, Ordering::Release);
    }
}

impl Plugin for SonoraVst {
    fn new(_host: HostCallback) -> Self {
        let params = Arc::new(SharedParams::new());
        let chain = VoiceChain::new(params.snapshot(), 48_000);
        Self {
            params,
            sample_rate: 48_000,
            chain,
        }
    }

    fn get_info(&self) -> Info {
        Info {
            name: "Sonora".to_string(),
            vendor: "Sonora".to_string(),
            unique_id: 0x536F_6E61, // "Sona"
            version: 2,
            inputs: 1,
            outputs: 1,
            parameters: PARAMS.len() as i32,
            category: Category::Effect,
            ..Default::default()
        }
    }

    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate.round() as u32;
        self.rebuild();
    }

    fn resume(&mut self) {
        self.rebuild();
    }

    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        if self.params.dirty.swap(false, Ordering::AcqRel) {
            self.chain.set_params(self.params.snapshot());
        }
        for (input, output) in buffer.zip() {
            for (i, o) in input.iter().zip(output.iter_mut()) {
                *o = self.chain.process_sample(*i);
            }
        }
    }

    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        self.params.clone()
    }
}

plugin_main!(SonoraVst);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_by_name_reach_the_chain() {
        let p = SharedParams::new();
        let idx = PARAMS
            .iter()
            .position(|s| s.name == "input_gain_db")
            .unwrap();
        p.set_parameter(idx as i32, 18.8);
        assert_eq!(p.get_parameter_name(idx as i32), "input_gain_db");
        assert!((p.snapshot().input_gain_db - 18.8).abs() < 1e-5);
        assert!(p.dirty.load(Ordering::Acquire));
    }
}
