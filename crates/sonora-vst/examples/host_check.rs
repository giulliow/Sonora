//! Carrega o módulo compilado como um host VST faria (igual ao Equalizer APO):
//! acha os parâmetros pelo nome, define valores e processa áudio.
//!
//! `cargo build --release -p sonora-vst && cargo run --release -p sonora-vst --example host_check -- <caminho da .dll/.so>`
#![allow(deprecated)]

use std::path::Path;
use std::sync::{Arc, Mutex};
use vst::host::{Host, PluginLoader};
use vst::plugin::Plugin;

struct H;
impl Host for H {}

fn main() {
    let path = std::env::args().nth(1).expect("caminho do módulo");
    let host = Arc::new(Mutex::new(H));
    let mut loader = PluginLoader::load(Path::new(&path), host).expect("carregar módulo");
    let mut inst = loader.instance().expect("instanciar");
    inst.init();
    let info = inst.get_info();
    println!(
        "{} | {} entrada(s), {} saída(s), {} parâmetros",
        info.name, info.inputs, info.outputs, info.parameters
    );
    inst.set_sample_rate(48_000.0);
    inst.set_block_size(480);
    inst.resume();

    let params = inst.get_parameter_object();
    let wanted = [
        ("input_gain_db", 12.0f32),
        ("comp_on", 1.0),
        ("limiter_ceiling_db", -1.0),
    ];
    for i in 0..info.parameters {
        let name = params.get_parameter_name(i);
        if let Some((_, v)) = wanted.iter().find(|(n, _)| *n == name) {
            params.set_parameter(i, *v);
            println!("  {name} = {}", params.get_parameter(i));
        }
    }

    let input: Vec<f32> = (0..480 * 100)
        .map(|i| (i as f32 * 0.05).sin() * 0.05)
        .collect();
    let mut output = vec![0.0f32; input.len()];
    for (inb, outb) in input.chunks(480).zip(output.chunks_mut(480)) {
        let mut buf = vst::host::HostBuffer::new(1, 1);
        let mut audio = buf.bind(&[inb], &mut [outb]);
        inst.process(&mut audio);
    }
    let peak_in = input.iter().fold(0f32, |m, s| m.max(s.abs()));
    let peak_out = output[24_000..].iter().fold(0f32, |m, s| m.max(s.abs()));
    println!(
        "pico entrada {:.1} dBFS → saída {:.1} dBFS",
        20.0 * peak_in.log10(),
        20.0 * peak_out.log10()
    );
    assert!(
        peak_out > peak_in * 1.5,
        "o ganho deveria ter subido o volume"
    );
    assert!(peak_out <= 0.892, "o limitador deveria segurar em -1 dBFS");
    println!("OK");
}
