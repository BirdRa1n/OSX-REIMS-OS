//! Versões de macOS suportadas e metadados associados.

use serde::{Deserialize, Serialize};

/// Versões de macOS oferecidas na criação de VMs (o padrão é a primeira).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MacosRelease {
    Ventura, // 13
    Sonoma,  // 14
    Sequoia, // 15
    Tahoe,   // 26
}

impl MacosRelease {
    pub const ALL: [MacosRelease; 4] = [
        MacosRelease::Ventura,
        MacosRelease::Sonoma,
        MacosRelease::Sequoia,
        MacosRelease::Tahoe,
    ];

    /// Número de versão "de marketing" do macOS.
    pub fn major(&self) -> u32 {
        match self {
            MacosRelease::Ventura => 13,
            MacosRelease::Sonoma => 14,
            MacosRelease::Sequoia => 15,
            MacosRelease::Tahoe => 26,
        }
    }

    pub fn code_name(&self) -> &'static str {
        match self {
            MacosRelease::Ventura => "Ventura",
            MacosRelease::Sonoma => "Sonoma",
            MacosRelease::Sequoia => "Sequoia",
            MacosRelease::Tahoe => "Tahoe",
        }
    }

    /// Nome que o `reims-ctl create --macos` aceita (`sonoma`).
    pub fn slug(&self) -> &'static str {
        match self {
            MacosRelease::Ventura => "ventura",
            MacosRelease::Sonoma => "sonoma",
            MacosRelease::Sequoia => "sequoia",
            MacosRelease::Tahoe => "tahoe",
        }
    }

    /// Faz parse de um nome de código ou número maior (ex.: "sonoma" ou "14").
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ventura" | "13" => Some(MacosRelease::Ventura),
            "sonoma" | "14" => Some(MacosRelease::Sonoma),
            "sequoia" | "15" => Some(MacosRelease::Sequoia),
            "tahoe" | "26" => Some(MacosRelease::Tahoe),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_and_major() {
        assert_eq!(MacosRelease::Sonoma.major(), 14);
        assert_eq!(MacosRelease::Sonoma.slug(), "sonoma");
        assert_eq!(
            MacosRelease::parse(MacosRelease::Tahoe.slug()),
            Some(MacosRelease::Tahoe)
        );
    }

    #[test]
    fn parse_names_and_numbers() {
        assert_eq!(MacosRelease::parse("Sonoma"), Some(MacosRelease::Sonoma));
        assert_eq!(MacosRelease::parse(" 15 "), Some(MacosRelease::Sequoia));
        assert_eq!(MacosRelease::parse("nope"), None);
    }
}
