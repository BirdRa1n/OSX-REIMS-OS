//! Configuração declarativa de uma VM macOS.

use crate::release::MacosRelease;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Recursos e caminhos de uma VM. Persistido como JSON no diretório da VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Nome único (também o diretório da VM).
    pub name: String,
    pub macos: MacosRelease,

    /// vCPUs.
    pub cpus: u32,
    /// RAM em MiB.
    pub ram_mb: u32,
    /// Tamanho do disco qcow2 em GiB (na criação).
    pub disk_gb: u32,

    /// Diretório-raiz da VM (contém disk.qcow2, OVMF_VARS, EFI, config.json).
    pub dir: PathBuf,

    /// Habilita a GPU virtual `reims-vgpu` (device `reims-vgpu-pci`).
    pub vgpu: bool,

    /// Chave OSK do AppleSMC. **Não** é distribuída por nós (propriedade da Apple);
    /// o usuário fornece. Vazio = QEMU não sobe macOS.
    #[serde(default)]
    pub applesmc_osk: String,
}

impl VmConfig {
    /// Config padrão sensata para uma nova VM.
    pub fn new(name: impl Into<String>, macos: MacosRelease, dir: PathBuf) -> Self {
        Self {
            name: name.into(),
            macos,
            cpus: 4,
            ram_mb: 8192,
            disk_gb: 128,
            dir,
            vgpu: true,
            applesmc_osk: String::new(),
        }
    }

    pub fn disk_path(&self) -> PathBuf {
        self.dir.join("disk.qcow2")
    }
    pub fn ovmf_vars_path(&self) -> PathBuf {
        self.dir.join("OVMF_VARS.fd")
    }
    pub fn opencore_path(&self) -> PathBuf {
        self.dir.join("OpenCore.qcow2")
    }
    pub fn config_path(&self) -> PathBuf {
        self.dir.join("config.json")
    }
}
