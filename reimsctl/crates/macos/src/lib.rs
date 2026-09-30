//! reimsctl-macos — download da mídia de instalação do macOS.
//!
//! Baixa a imagem de **Recovery** direto dos servidores da Apple (mesma
//! abordagem do `fetch-macOS`/gibMacOS do ecossistema OSX-KVM). Nenhuma imagem
//! é redistribuída por este projeto — o download acontece na máquina do usuário.

use anyhow::Result;
use reimsctl_core::release::MacosRelease;
use std::path::{Path, PathBuf};

/// Resultado de um download: caminho do `BaseSystem.dmg`/`.img` obtido.
#[derive(Debug, Clone)]
pub struct RecoveryImage {
    pub release: MacosRelease,
    pub path: PathBuf,
}

/// Baixa a imagem de Recovery da versão pedida para `dest_dir`.
///
/// TODO(fase-1): implementar o protocolo do servidor de OTA/Recovery da Apple
/// (catálogo de sucatalog + chunklist), com verificação de integridade. Por ora
/// apenas valida o destino.
pub fn fetch_recovery(release: MacosRelease, dest_dir: &Path) -> Result<RecoveryImage> {
    anyhow::ensure!(
        dest_dir.is_dir(),
        "destino não é um diretório: {}",
        dest_dir.display()
    );
    let _ = release;
    anyhow::bail!("fetch_recovery: não implementado (fase-1) — ver docs/ROADMAP.md")
}
