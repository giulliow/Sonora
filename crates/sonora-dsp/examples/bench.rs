//! Mede quanto custa processar 60 s de voz com a cadeia completa.
//! `cargo run --release -p sonora-dsp --example bench`
use sonora_dsp::chain::{ChainParams, VoiceChain};
use std::time::Instant;

fn main() {
    let sr = 48_000;
    let mut x: Vec<f32> = (0..sr * 60)
        .map(|i| (i as f32 * 0.03).sin() * 0.2)
        .collect();
    let p = ChainParams {
        input_gain_db: 12.0,
        hpf_on: true,
        denoise_on: true,
        gate_on: true,
        eq_on: true,
        deess_on: true,
        comp_on: true,
        ..Default::default()
    };
    let mut c = VoiceChain::new(p, sr as u32);
    let t = Instant::now();
    for block in x.chunks_mut(480) {
        c.process(block);
    }
    let secs = t.elapsed().as_secs_f64();
    println!(
        "60 s de áudio em {:.3} s ({:.2}% de um núcleo)",
        secs,
        secs / 60.0 * 100.0
    );
}
