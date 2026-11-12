//! O que o Sonora lembra entre uma abertura e outra.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sonora_dsp::calibration::Recommendation;
use sonora_dsp::chain::ChainParams;
use sonora_dsp::mic::MicType;
use sonora_dsp::presets::{build, SimpleControls};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Quatro controles simples.
    #[default]
    Simple,
    /// Todos os números editáveis.
    Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub device_name: Option<String>,
    pub mic_type: Option<MicType>,
    /// Última calibração. Só uma sugestão: a pessoa escolhe se usa.
    pub recommendation: Option<Recommendation>,
    /// Usar o ajuste de volume e a porta de ruído da calibração.
    pub use_calibration: bool,
    pub mode: Mode,
    pub simple: SimpleControls,
    /// Valores do modo avançado (quando a pessoa já mexeu nele).
    pub advanced: Option<ChainParams>,
    /// Tratamento ligado no Windows.
    pub enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            device_name: None,
            mic_type: None,
            recommendation: None,
            use_calibration: true,
            mode: Mode::Simple,
            simple: SimpleControls::default(),
            advanced: None,
            enabled: true,
        }
    }
}

impl Settings {
    /// A cadeia que vale de verdade, a partir do modo escolhido.
    pub fn effective(&self) -> ChainParams {
        let mut p = match (self.mode, self.advanced) {
            (Mode::Advanced, Some(adv)) => adv,
            _ => self.simple_params(),
        };
        p.enabled = self.enabled;
        p.sanitized()
    }

    /// O que os controles simples produzem (também é o ponto de partida do avançado).
    pub fn simple_params(&self) -> ChainParams {
        let rec = self
            .recommendation
            .as_ref()
            .filter(|_| self.use_calibration);
        build(self.mic_type, rec, &self.simple)
    }

    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| format!("não foi possível salvar as configurações: {e}"))
    }
}

pub fn settings_path(config_dir: PathBuf) -> PathBuf {
    config_dir.join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advanced_overrides_simple() {
        let adv = ChainParams {
            input_gain_db: 7.0,
            ..Default::default()
        };
        let s = Settings {
            mode: Mode::Advanced,
            advanced: Some(adv),
            ..Default::default()
        };
        assert_eq!(s.effective().input_gain_db, 7.0);
        let s = Settings {
            mode: Mode::Simple,
            ..s
        };
        assert_ne!(s.effective().input_gain_db, 7.0);
    }

    #[test]
    fn disabled_flag_wins() {
        let s = Settings {
            enabled: false,
            ..Default::default()
        };
        assert!(!s.effective().enabled);
    }

    #[test]
    fn old_or_partial_json_loads() {
        let s: Settings =
            serde_json::from_str(r#"{"device_name":"Microfone (Shure MV7)"}"#).unwrap();
        assert_eq!(s.device_name.as_deref(), Some("Microfone (Shure MV7)"));
        assert!(s.enabled);
    }
}
