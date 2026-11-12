//! Integração com o Equalizer APO: a "tomada" dentro do sistema de áudio
//! do Windows onde o módulo do Sonora fica plugado.
//!
//! O Sonora escreve dois arquivos na pasta de configuração do Equalizer APO:
//! - `sonora.txt`: o módulo e todos os números da cadeia;
//! - um bloco em `config.txt` que inclui o `sonora.txt`.
//!
//! O Equalizer APO percebe a mudança e recarrega sozinho. Nada do Sonora
//! precisa ficar aberto.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sonora_dsp::chain::{ChainParams, PARAMS};

pub const SONORA_FILE: &str = "sonora.txt";
const BLOCK_BEGIN: &str = "# >>> Sonora: não edite este bloco (é atualizado pelo app) >>>";
const BLOCK_END: &str = "# <<< Sonora <<<";
const BACKUP_SUFFIX: &str = ".antes-do-sonora";

/// Configuração que vem de fábrica no Equalizer APO (só exemplo, sem efeito útil).
const FACTORY_CONFIG: &str = "Preamp: -6 dB\nInclude: example.txt\nGraphicEQ: 25 0; 40 0; 63 0; 100 0; 160 0; 250 0; 400 0; 630 0; 1000 0; 1600 0; 2500 0; 4000 0; 6300 0; 10000 0; 16000 0";

#[derive(Debug, Clone, Serialize)]
pub struct EqApoStatus {
    /// O Equalizer APO está instalado neste PC.
    pub installed: bool,
    pub config_dir: Option<String>,
    /// Caminho do "Configurator" (onde se escolhe o microfone).
    pub configurator: Option<String>,
    /// O módulo do Sonora está no lugar certo.
    pub plugin_found: bool,
    /// O config.txt já inclui o Sonora.
    pub linked: bool,
    /// O config.txt tem outros ajustes além do Sonora, que também valem para o microfone.
    pub has_other_settings: bool,
    /// O tratamento está ligado (o módulo está ativo no sonora.txt).
    pub processing_on: bool,
}

/// Monta o conteúdo do `sonora.txt`.
pub fn render_sonora_txt(plugin_path: &Path, params: &ChainParams) -> String {
    let mut out = String::new();
    out.push_str("# Gerado pelo Sonora. Mudanças aqui são sobrescritas pelo app.\n");
    out.push_str("# Só vale para microfones (o estágio \"capture\" do Equalizer APO).\n");
    out.push_str("Stage: capture\n");
    if params.enabled {
        out.push_str(&format!("VSTPlugin: Library \"{}\"", plugin_path.display()));
        for spec in PARAMS {
            out.push_str(&format!(
                " \"{}\" {}",
                spec.name,
                fmt_num((spec.get)(params))
            ));
        }
        out.push('\n');
    } else {
        out.push_str("# Tratamento desligado: o som do microfone passa sem mudanças.\n");
    }
    out.push_str("Stage: post-mix capture\n");
    out
}

fn fmt_num(v: f32) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".into()
    } else {
        s.to_string()
    }
}

fn block() -> String {
    format!("{BLOCK_BEGIN}\nInclude: {SONORA_FILE}\n{BLOCK_END}\n")
}

fn normalized(s: &str) -> String {
    s.replace("\r\n", "\n").trim().to_string()
}

/// O que fazer com o `config.txt` para incluir o Sonora.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigPlan {
    /// Já inclui o Sonora.
    AlreadyLinked,
    /// Era a configuração de fábrica: salvar uma cópia e trocar pelo bloco do Sonora.
    ReplaceFactory(String),
    /// Tem ajustes da pessoa: manter tudo e acrescentar o bloco no fim.
    Append(String),
}

pub fn plan_config(current: &str) -> ConfigPlan {
    if current.contains(BLOCK_BEGIN) {
        return ConfigPlan::AlreadyLinked;
    }
    let n = normalized(current);
    if n.is_empty() || n == normalized(FACTORY_CONFIG) {
        ConfigPlan::ReplaceFactory(block())
    } else {
        let mut s = current.to_string();
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push('\n');
        s.push_str(&block());
        ConfigPlan::Append(s)
    }
}

/// Tem algo além do bloco do Sonora no config.txt?
pub fn has_other_settings(current: &str) -> bool {
    let mut inside = false;
    for line in current.lines().map(str::trim) {
        if line == BLOCK_BEGIN {
            inside = true;
        } else if line == BLOCK_END {
            inside = false;
        } else if !inside && !line.is_empty() && !line.starts_with('#') {
            return true;
        }
    }
    false
}

/// Onde o Equalizer APO guarda a configuração.
pub fn config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SONORA_EQAPO_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    platform::config_dir()
}

fn configurator_path() -> Option<PathBuf> {
    platform::install_dir()
        .map(|d| d.join("Configurator.exe"))
        .filter(|p| p.exists())
        .or_else(|| {
            platform::install_dir()
                .map(|d| d.join("DeviceSelector.exe"))
                .filter(|p| p.exists())
        })
}

pub fn status(plugin_path: &Path) -> EqApoStatus {
    let dir = config_dir().filter(|d| d.join("config.txt").exists());
    let config = dir
        .as_ref()
        .and_then(|d| fs::read_to_string(d.join("config.txt")).ok());
    let sonora = dir
        .as_ref()
        .and_then(|d| fs::read_to_string(d.join(SONORA_FILE)).ok());
    EqApoStatus {
        installed: dir.is_some(),
        config_dir: dir.as_ref().map(|d| d.display().to_string()),
        configurator: configurator_path().map(|p| p.display().to_string()),
        plugin_found: plugin_path.exists(),
        linked: config.as_deref().is_some_and(|c| c.contains(BLOCK_BEGIN)),
        has_other_settings: config.as_deref().is_some_and(has_other_settings),
        processing_on: sonora.as_deref().is_some_and(|s| s.contains("VSTPlugin:")),
    }
}

/// Escreve o `sonora.txt` e garante que o `config.txt` o inclui.
pub fn apply(plugin_path: &Path, params: &ChainParams) -> Result<(), String> {
    let dir = config_dir()
        .filter(|d| d.join("config.txt").exists())
        .ok_or("o Equalizer APO não está instalado")?;
    if params.enabled && !plugin_path.exists() {
        return Err(format!(
            "o módulo do Sonora não foi encontrado em {} (reinstale o Sonora)",
            plugin_path.display()
        ));
    }
    let write = |path: PathBuf, content: &str| {
        fs::write(&path, content).map_err(|e| {
            format!(
                "não foi possível salvar {}: {e}. Tente abrir o Sonora como administrador.",
                path.display()
            )
        })
    };

    write(
        dir.join(SONORA_FILE),
        &render_sonora_txt(plugin_path, params),
    )?;

    let config_path = dir.join("config.txt");
    let current = fs::read_to_string(&config_path).unwrap_or_default();
    match plan_config(&current) {
        ConfigPlan::AlreadyLinked => {}
        ConfigPlan::ReplaceFactory(new) => {
            let backup = dir.join(format!("config.txt{BACKUP_SUFFIX}"));
            if !backup.exists() {
                write(backup, &current)?;
            }
            write(config_path, &new)?;
        }
        ConfigPlan::Append(new) => write(config_path, &new)?,
    }
    Ok(())
}

#[cfg(windows)]
mod platform {
    use std::path::PathBuf;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
    use winreg::RegKey;

    fn key() -> Option<RegKey> {
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags("SOFTWARE\\EqualizerAPO", KEY_READ | KEY_WOW64_64KEY)
            .ok()
    }

    pub fn config_dir() -> Option<PathBuf> {
        key()
            .and_then(|k| k.get_value::<String, _>("ConfigPath").ok())
            .map(PathBuf::from)
            .or_else(|| install_dir().map(|d| d.join("config")))
    }

    pub fn install_dir() -> Option<PathBuf> {
        key()
            .and_then(|k| k.get_value::<String, _>("InstallPath").ok())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("ProgramFiles")
                    .ok()
                    .map(|p| PathBuf::from(p).join("EqualizerAPO"))
                    .filter(|p| p.exists())
            })
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::PathBuf;

    // O Equalizer APO só existe no Windows. No Mac o caminho será outro.
    pub fn config_dir() -> Option<PathBuf> {
        None
    }

    pub fn install_dir() -> Option<PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sonora_txt_has_every_param_by_name() {
        let p = ChainParams {
            input_gain_db: 18.8,
            comp_on: true,
            ..Default::default()
        };
        let txt = render_sonora_txt(Path::new("C:\\Program Files\\Sonora\\sonora_vst.dll"), &p);
        assert!(txt.contains("Stage: capture\n"));
        assert!(txt.contains("VSTPlugin: Library \"C:\\Program Files\\Sonora\\sonora_vst.dll\""));
        assert!(txt.contains("\"input_gain_db\" 18.8"));
        assert!(txt.contains("\"comp_on\" 1"));
        assert!(txt.contains("\"hpf_on\" 0"));
        for spec in PARAMS {
            assert!(
                txt.contains(&format!("\"{}\" ", spec.name)),
                "{}",
                spec.name
            );
        }
    }

    #[test]
    fn disabled_writes_no_plugin() {
        let p = ChainParams {
            enabled: false,
            ..Default::default()
        };
        let txt = render_sonora_txt(Path::new("x.dll"), &p);
        assert!(!txt.contains("VSTPlugin"));
    }

    #[test]
    fn numbers_are_compact() {
        assert_eq!(fmt_num(18.8), "18.8");
        assert_eq!(fmt_num(1.0), "1");
        assert_eq!(fmt_num(-0.0), "0");
        assert_eq!(fmt_num(100.0), "100");
        assert_eq!(fmt_num(-24.0), "-24");
    }

    #[test]
    fn factory_config_is_replaced() {
        let factory = format!("{FACTORY_CONFIG}\r\n");
        match plan_config(&factory) {
            ConfigPlan::ReplaceFactory(new) => {
                assert!(new.contains("Include: sonora.txt"));
                assert!(!new.contains("Preamp: -6 dB"));
                assert!(!has_other_settings(&new));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn custom_config_is_kept() {
        let custom = "Preamp: -3 dB\nInclude: meu_fone.txt";
        match plan_config(custom) {
            ConfigPlan::Append(new) => {
                assert!(new.starts_with(custom));
                assert!(new.contains("Include: sonora.txt"));
                assert!(has_other_settings(&new));
                assert_eq!(plan_config(&new), ConfigPlan::AlreadyLinked);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn apply_end_to_end_in_temp_dir() {
        let dir = std::env::temp_dir().join(format!("sonora-eqapo-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.txt"), FACTORY_CONFIG).unwrap();
        let plugin = dir.join("sonora_vst.dll");
        fs::write(&plugin, b"").unwrap();
        std::env::set_var("SONORA_EQAPO_CONFIG_DIR", &dir);

        apply(&plugin, &ChainParams::default()).unwrap();
        let st = status(&plugin);
        assert!(st.installed && st.linked && st.plugin_found && st.processing_on);
        assert!(!st.has_other_settings);
        assert!(dir.join("config.txt.antes-do-sonora").exists());

        apply(
            &plugin,
            &ChainParams {
                enabled: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!status(&plugin).processing_on);

        std::env::remove_var("SONORA_EQAPO_CONFIG_DIR");
        fs::remove_dir_all(&dir).ok();
    }
}
