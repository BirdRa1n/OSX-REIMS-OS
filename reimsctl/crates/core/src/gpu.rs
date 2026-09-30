//! Detecção e classificação da GPU do host.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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

    /// Nome curto em minúsculas (para scripts do provisioner).
    pub fn as_str(&self) -> &'static str {
        match self {
            GpuVendor::Intel => "intel",
            GpuVendor::Amd => "amd",
            GpuVendor::Nvidia => "nvidia",
            GpuVendor::Unknown => "unknown",
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

/// Diretório sysfs padrão com os devices PCI.
const PCI_DEVICES: &str = "/sys/bus/pci/devices";

/// Base class PCI de controladores de display (0x03).
const PCI_CLASS_DISPLAY: u32 = 0x03;

/// Detecta a GPU primária do host lendo o sysfs PCI.
pub fn detect_primary() -> GpuVendor {
    detect_in(Path::new(PCI_DEVICES))
}

/// Igual a [`detect_primary`], mas sobre um diretório de devices arbitrário
/// (permite testar com um fixture). Retorna o primeiro device de classe display
/// com vendor reconhecido; senão `Unknown`.
pub fn detect_in(devices_dir: &Path) -> GpuVendor {
    let entries = match fs::read_dir(devices_dir) {
        Ok(e) => e,
        Err(_) => return GpuVendor::Unknown,
    };
    let mut fallback = GpuVendor::Unknown;
    for entry in entries.flatten() {
        let dev = entry.path();
        let (Some(class), Some(vendor)) =
            (read_hex(&dev.join("class")), read_hex(&dev.join("vendor")))
        else {
            continue;
        };
        // base class = byte mais alto do class code de 24 bits.
        if (class >> 16) & 0xff != PCI_CLASS_DISPLAY {
            continue;
        }
        let v = GpuVendor::from_pci_vendor_id(vendor as u16);
        if v != GpuVendor::Unknown {
            return v;
        }
        fallback = v;
    }
    fallback
}

/// Lê um valor hex do sysfs (ex.: "0x030000\n") como u32.
fn read_hex(path: &Path) -> Option<u32> {
    let s = fs::read_to_string(path).ok()?;
    let s = s.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    u32::from_str_radix(s, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_devices() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("reimsctl-gpu-{}-{}", std::process::id(), nanos));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn mkdev(root: &Path, slot: &str, class: &str, vendor: &str) {
        let d = root.join(slot);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("class"), format!("{class}\n")).unwrap();
        fs::write(d.join("vendor"), format!("{vendor}\n")).unwrap();
    }

    #[test]
    fn vendor_id_mapping() {
        assert_eq!(GpuVendor::from_pci_vendor_id(0x8086), GpuVendor::Intel);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x1002), GpuVendor::Amd);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x10de), GpuVendor::Nvidia);
        assert_eq!(GpuVendor::from_pci_vendor_id(0xffff), GpuVendor::Unknown);
    }

    #[test]
    fn detects_display_device_skips_others() {
        let root = temp_devices();
        // nomes sem ':' (ilegal em arquivos no Windows); a detecção só olha
        // os arquivos class/vendor, não o nome do diretório.
        // device de rede (classe 0x02) deve ser ignorado
        mkdev(&root, "dev-net", "0x020000", "0x8086");
        // GPU NVIDIA (classe display 0x03)
        mkdev(&root, "dev-gpu", "0x030000", "0x10de");
        assert_eq!(detect_in(&root), GpuVendor::Nvidia);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn missing_dir_is_unknown() {
        assert_eq!(
            detect_in(Path::new("/nope/does/not/exist")),
            GpuVendor::Unknown
        );
    }
}
