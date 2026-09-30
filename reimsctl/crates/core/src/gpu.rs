//! Detecção e classificação da GPU do host.

use serde::{Deserialize, Serialize};

/// Família da GPU do host, que determina o driver e o ICD Vulkan usados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    /// i915/xe + Mesa ANV
    Intel,
    /// amdgpu + Mesa RADV
    Amd,
    /// driver proprietário NVIDIA (opt-in, non-free)
    Nvidia,
    Unknown,
}

impl GpuVendor {
    /// ID de vendor PCI (`lspci`/sysfs) → família.
    pub fn from_pci_vendor_id(id: u16) -> Self {
        match id {
            0x8086 => GpuVendor::Intel,
            0x1002 | 0x1022 => GpuVendor::Amd,
            0x10de => GpuVendor::Nvidia,
            _ => GpuVendor::Unknown,
        }
    }

    /// Nome do ICD/driver Vulkan esperado (informativo).
    pub fn vulkan_driver(&self) -> &'static str {
        match self {
            GpuVendor::Intel => "Mesa ANV",
            GpuVendor::Amd => "Mesa RADV",
            GpuVendor::Nvidia => "NVIDIA proprietary",
            GpuVendor::Unknown => "unknown",
        }
    }

    /// NVIDIA exige autorização explícita do usuário (driver non-free).
    pub fn requires_optin(&self) -> bool {
        matches!(self, GpuVendor::Nvidia)
    }
}

/// Detecta a GPU primária do host.
///
/// TODO(fase-2): implementar lendo `/sys/bus/pci/devices/*/{class,vendor}` e
/// filtrando por classe de display (0x0300xx). Por ora retorna `Unknown`.
pub fn detect_primary() -> GpuVendor {
    GpuVendor::Unknown
}
