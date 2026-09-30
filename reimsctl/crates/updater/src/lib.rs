//! reimsctl-updater — atualizações **pós-instalação** do OSX-REIMS-OS.
//!
//! Só disponível depois que o sistema foi instalado no disco. Atualiza: o
//! próprio `reimsctl`, o `reims-vgpu` (rebuild) e as bases de OpenCore. Canal
//! candidato: repositório APT próprio assinado (ver `docs/ARCHITECTURE.md` §6).

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct UpdateStatus {
    pub current: String,
    pub latest: Option<String>,
}

/// Consulta se há atualização disponível.
///
/// TODO(fase-3): falar com o repo APT/releases e comparar versões.
pub fn check() -> Result<UpdateStatus> {
    Ok(UpdateStatus {
        current: env!("CARGO_PKG_VERSION").to_string(),
        latest: None,
    })
}

/// Aplica a atualização disponível.
///
/// TODO(fase-3): `apt update && apt install` do repo assinado + rebuild vgpu.
pub fn apply() -> Result<()> {
    anyhow::bail!("apply: não implementado (fase-3)")
}
