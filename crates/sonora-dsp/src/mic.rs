//! Descobre o tipo de microfone pelo nome que o sistema mostra.
//!
//! O Windows mostra nomes como "Microfone (Shure MV7)". Procuramos pedaços
//! conhecidos desse nome. Se nada bater, o app pergunta para a pessoa.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicType {
    /// Dinâmico: pega pouco barulho do quarto, mas precisa de mais ganho
    /// e de falar perto. Ex.: Shure MV7, SM7B.
    Dynamic,
    /// Condensador: muito sensível e detalhado, mas pega teclado,
    /// ventilador e eco. Ex.: Blue Yeti, HyperX QuadCast.
    Condenser,
    /// Microfone na haste de um fone de ouvido.
    Headset,
    /// Microfone do notebook ou da webcam.
    BuiltIn,
    /// Microfone de lapela (de prender na roupa).
    Lavalier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Reconhecemos o modelo exato.
    High,
    /// Só um palpite pelo tipo de dispositivo; vale confirmar com a pessoa.
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MicGuess {
    pub mic_type: MicType,
    /// Nome bonito do modelo, quando sabemos.
    pub model: Option<&'static str>,
    pub confidence: Confidence,
}

struct Known {
    needle: &'static str,
    mic_type: MicType,
    model: Option<&'static str>,
    confidence: Confidence,
}

const fn k(needle: &'static str, mic_type: MicType, model: &'static str) -> Known {
    Known {
        needle,
        mic_type,
        model: Some(model),
        confidence: Confidence::High,
    }
}

const fn hint(needle: &'static str, mic_type: MicType) -> Known {
    Known {
        needle,
        mic_type,
        model: None,
        confidence: Confidence::Low,
    }
}

/// Ordem importa: modelos específicos antes de palavras genéricas.
const KNOWN: &[Known] = &[
    // Dinâmicos
    k("mv7", MicType::Dynamic, "Shure MV7"),
    k("mv6", MicType::Dynamic, "Shure MV6"),
    k("sm7b", MicType::Dynamic, "Shure SM7B"),
    k("q2u", MicType::Dynamic, "Samson Q2U"),
    k("q9u", MicType::Dynamic, "Samson Q9U"),
    k("atr2100", MicType::Dynamic, "Audio-Technica ATR2100x"),
    k("podmic", MicType::Dynamic, "Rode PodMic USB"),
    k("pd200", MicType::Dynamic, "Maono PD200X"),
    k("ampligame am8", MicType::Dynamic, "Fifine AmpliGame AM8"),
    k("procaster", MicType::Dynamic, "Rode Procaster"),
    // Condensadores
    k("yeti", MicType::Condenser, "Blue Yeti"),
    k("snowball", MicType::Condenser, "Blue Snowball"),
    k("quadcast", MicType::Condenser, "HyperX QuadCast"),
    k("solocast", MicType::Condenser, "HyperX SoloCast"),
    k("nt-usb", MicType::Condenser, "Rode NT-USB"),
    k("wave:", MicType::Condenser, "Elgato Wave"),
    k("at2020", MicType::Condenser, "Audio-Technica AT2020"),
    k("seiren", MicType::Condenser, "Razer Seiren"),
    k("mv5", MicType::Condenser, "Shure MV5"),
    hint("fifine", MicType::Condenser),
    // Lapela
    k("dji mic", MicType::Lavalier, "DJI Mic"),
    k("rode wireless", MicType::Lavalier, "Rode Wireless GO"),
    hint("lavalier", MicType::Lavalier),
    hint("lapela", MicType::Lavalier),
    // Headsets
    k("airpods", MicType::Headset, "AirPods"),
    k("arctis", MicType::Headset, "SteelSeries Arctis"),
    k("hyperx cloud", MicType::Headset, "HyperX Cloud"),
    k("blackshark", MicType::Headset, "Razer BlackShark"),
    k("kraken", MicType::Headset, "Razer Kraken"),
    hint("headset", MicType::Headset),
    hint("hands-free", MicType::Headset),
    hint("viva-voz", MicType::Headset),
    // Embutidos
    k("c920", MicType::BuiltIn, "Logitech C920"),
    hint("webcam", MicType::BuiltIn),
    hint("matriz de microfone", MicType::BuiltIn),
    hint("microphone array", MicType::BuiltIn),
    hint("built-in", MicType::BuiltIn),
    hint("macbook", MicType::BuiltIn),
    hint("integrado", MicType::BuiltIn),
];

/// Tenta reconhecer o microfone pelo nome do dispositivo.
pub fn guess_from_name(device_name: &str) -> Option<MicGuess> {
    let name = device_name.to_lowercase();
    KNOWN
        .iter()
        .find(|kn| name.contains(kn.needle))
        .map(|kn| MicGuess {
            mic_type: kn.mic_type,
            model: kn.model,
            confidence: kn.confidence,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_mv7_windows_name() {
        let g = guess_from_name("Microfone (Shure MV7)").unwrap();
        assert_eq!(g.mic_type, MicType::Dynamic);
        assert_eq!(g.model, Some("Shure MV7"));
        assert_eq!(g.confidence, Confidence::High);
    }

    #[test]
    fn recognizes_yeti_and_array() {
        assert_eq!(
            guess_from_name("Microphone (Yeti Stereo Microphone)")
                .unwrap()
                .mic_type,
            MicType::Condenser
        );
        let g = guess_from_name("Matriz de Microfone (Realtek(R) Audio)").unwrap();
        assert_eq!(g.mic_type, MicType::BuiltIn);
        assert_eq!(g.confidence, Confidence::Low);
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(guess_from_name("Microfone (USB Audio Device)"), None);
    }
}
