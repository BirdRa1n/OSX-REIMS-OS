//! Nomes, versões e caminhos do appliance num lugar só.
//!
//! O projeto vai mudar de nome: tudo que aparece para o usuário (nome do
//! produto, usuário do sistema, diretórios) sai daqui, para a troca ser uma
//! edição só. Os scripts de shell leem os equivalentes de `/usr/lib/reims/brand.sh`.

use std::path::PathBuf;

/// Nome do produto, como aparece nas telas.
pub const PRODUCT: &str = "OSX-REIMS-OS";

/// Versão do appliance (a mesma do workspace Rust).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Usuário da sessão do appliance (criado pelo instalador).
pub const USER: &str = "reims";

/// Estado do sistema (root): first setup, gráficos, hardware.
pub fn system_state_dir() -> PathBuf {
    env_path("REIMS_SYSTEM_STATE", "/var/lib/reims")
}

/// Checkout do Reims vGPU.
pub fn reims_vgpu_dir() -> PathBuf {
    env_path("REIMS_VGPU_DIR", "/opt/reims-vgpu")
}

/// Estado do usuário (licença, preparo, VMs, logs): `$XDG_STATE_HOME/reims-os`.
pub fn user_state_dir() -> PathBuf {
    if let Some(v) = std::env::var_os("REIMS_USER_STATE") {
        return PathBuf::from(v);
    }
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".local/state"));
    base.join("reims-os")
}

/// Imagens de instalador completo oferecidas na criação de VMs.
pub fn full_installers_dir() -> PathBuf {
    user_state_dir().join("full")
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn env_path(var: &str, default: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default))
}
