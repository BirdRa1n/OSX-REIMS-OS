//! Versões de macOS suportadas e metadados associados.

use serde::{Deserialize, Serialize};

/// Versões de macOS que o appliance sabe provisionar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MacosRelease {
    Ventura,  // 13
    Sonoma,   // 14
    Sequoia,  // 15
    Tahoe,    // 26
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

    /// Nome do "rail" (árvore de EFI/OpenCore + snapshots) desta versão.
    pub fn rail(&self) -> String {
        format!("macos-{}", self.major())
    }
}
