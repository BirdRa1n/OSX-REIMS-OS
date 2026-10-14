//! Detecção e validação do hardware: CPU, virtualização, KVM, IOMMU e cada GPU.
//!
//! É o mesmo código que o first setup usa (`reimsctl hw ...`): `detect` lê o
//! sysfs, `plan` diz o que instalar do repositório do Debian e por quê, e
//! `validate` confere que a GPU do monitor responde ao Vulkan de verdade — não
//! uma renderização por software (llvmpipe).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Família da GPU, que decide o driver e o ICD Vulkan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    Intel,
    Amd,
    Nvidia,
    Unknown,
}

impl GpuVendor {
    pub fn from_pci_vendor_id(id: u16) -> Self {
        match id {
            0x8086 => GpuVendor::Intel,
            0x1002 | 0x1022 => GpuVendor::Amd,
            0x10de => GpuVendor::Nvidia,
            _ => GpuVendor::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            GpuVendor::Intel => "intel",
            GpuVendor::Amd => "amd",
            GpuVendor::Nvidia => "nvidia",
            GpuVendor::Unknown => "unknown",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            GpuVendor::Intel => "Intel",
            GpuVendor::Amd => "AMD",
            GpuVendor::Nvidia => "NVIDIA",
            GpuVendor::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gpu {
    /// Endereço PCI (ex.: `0000:00:02.0`).
    pub slot: String,
    pub vendor_id: u16,
    pub device_id: u16,
    pub vendor: GpuVendor,
    pub name: String,
    /// Driver do kernel ligado ao device (`i915`, `xe`, `amdgpu`, `nvidia`…).
    pub driver: Option<String>,
    /// A GPU em que o monitor está (`boot_vga`).
    pub display: bool,
}

impl Gpu {
    pub fn id(&self) -> String {
        format!("{:04x}:{:04x}", self.vendor_id, self.device_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cpu {
    pub model: String,
    pub vendor: String,
    /// `VT-x`, `AMD-V`, ou `None` sem virtualização de hardware.
    pub virt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Iommu {
    /// O firmware anuncia IOMMU (tabela ACPI DMAR/IVRS).
    pub firmware: bool,
    /// O kernel está com IOMMU ativo.
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hardware {
    pub cpu: Cpu,
    pub kvm: bool,
    pub iommu: Iommu,
    pub gpus: Vec<Gpu>,
}

impl Hardware {
    /// A GPU do monitor (ou a primeira, se nenhuma é `boot_vga`).
    pub fn display_gpu(&self) -> Option<&Gpu> {
        self.gpus
            .iter()
            .find(|g| g.display)
            .or_else(|| self.gpus.first())
    }

    pub fn has_vendor(&self, v: GpuVendor) -> bool {
        self.gpus.iter().any(|g| g.vendor == v)
    }
}

/// Lê o hardware deste computador.
pub fn detect() -> Hardware {
    let mut hw = detect_from(Path::new("/"));
    for g in &mut hw.gpus {
        if let Some(n) = lspci_name(&g.slot) {
            g.name = n;
        }
    }
    hw
}

/// Igual a [`detect`], com outra raiz de sistema de arquivos (testes).
pub fn detect_from(root: &Path) -> Hardware {
    let cpuinfo = fs::read_to_string(root.join("proc/cpuinfo")).unwrap_or_default();
    let cpu = parse_cpu(&cpuinfo);
    let kvm = root.join("dev/kvm").exists();
    let acpi = root.join("sys/firmware/acpi/tables");
    let iommu = Iommu {
        firmware: acpi.join("DMAR").exists() || acpi.join("IVRS").exists(),
        enabled: dir_not_empty(&root.join("sys/class/iommu")),
    };
    let gpus = detect_gpus(&root.join("sys/bus/pci/devices"));
    Hardware {
        cpu,
        kvm,
        iommu,
        gpus,
    }
}

fn parse_cpu(cpuinfo: &str) -> Cpu {
    let field = |k: &str| {
        cpuinfo
            .lines()
            .find(|l| l.starts_with(k))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.trim().to_string())
            .unwrap_or_default()
    };
    let flags = field("flags");
    let has = |f: &str| flags.split_whitespace().any(|x| x == f);
    let virt = if has("vmx") {
        Some("VT-x".into())
    } else if has("svm") {
        Some("AMD-V".into())
    } else {
        None
    };
    Cpu {
        model: field("model name"),
        vendor: field("vendor_id"),
        virt,
    }
}

fn detect_gpus(devices: &Path) -> Vec<Gpu> {
    let Ok(entries) = fs::read_dir(devices) else {
        return Vec::new();
    };
    let mut gpus = Vec::new();
    for e in entries.flatten() {
        let dev = e.path();
        let (Some(class), Some(vendor), Some(device)) = (
            read_hex(&dev.join("class")),
            read_hex(&dev.join("vendor")),
            read_hex(&dev.join("device")),
        ) else {
            continue;
        };
        // classe base 0x03 = controlador de display.
        if (class >> 16) & 0xff != 0x03 {
            continue;
        }
        let vendor_id = vendor as u16;
        let device_id = device as u16;
        let driver = fs::read_link(dev.join("driver"))
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .or_else(|| {
                // fixtures de teste (e Windows) não têm symlink: aceita um arquivo.
                fs::read_to_string(dev.join("driver_name"))
                    .ok()
                    .map(|s| s.trim().to_string())
            });
        let display = fs::read_to_string(dev.join("boot_vga"))
            .map(|s| s.trim() == "1")
            .unwrap_or(false);
        let v = GpuVendor::from_pci_vendor_id(vendor_id);
        gpus.push(Gpu {
            slot: e.file_name().to_string_lossy().replace('_', ":"),
            vendor_id,
            device_id,
            vendor: v,
            name: format!("{} GPU [{:04x}:{:04x}]", v.label(), vendor_id, device_id),
            driver,
            display,
        });
    }
    gpus.sort_by(|a, b| a.slot.cmp(&b.slot));
    gpus
}

/// Nome legível da GPU via `lspci -mm` (`"Intel Corporation" "Raptor Lake [UHD 770]"`).
fn lspci_name(slot: &str) -> Option<String> {
    let out = crate::sys::output("lspci", &["-mm", "-s", slot])?;
    let fields: Vec<&str> = out.split('"').collect();
    // 0: slot, 1: classe, 3: vendor, 5: device
    let vendor = fields.get(3)?.trim();
    let device = fields.get(5)?.trim();
    Some(format!("{vendor} {device}"))
}

fn read_hex(path: &Path) -> Option<u32> {
    let s = fs::read_to_string(path).ok()?;
    let s = s.trim();
    u32::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok()
}

fn dir_not_empty(p: &Path) -> bool {
    fs::read_dir(p)
        .map(|mut d| d.next().is_some())
        .unwrap_or(false)
}

// ------------------------------------------------------------------ plano

/// O que o first setup instala, por quê, e o que muda no kernel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// Pacotes do Debian (sem os da NVIDIA, que dependem do `nvidia-detect`).
    pub packages: Vec<String>,
    /// Pacotes da NVIDIA, instalados só com autorização.
    pub nvidia_packages: Vec<String>,
    /// Módulos do kernel esperados para as GPUs.
    pub modules: Vec<String>,
    /// Parâmetros de kernel propostos (IOMMU, opcional).
    pub iommu_args: Vec<String>,
    /// Linhas de explicação, uma por GPU.
    pub reasons: Vec<String>,
}

/// Pacotes que entram em qualquer computador.
pub const BASE_PACKAGES: &[&str] = &[
    "libvulkan1",
    "mesa-vulkan-drivers",
    "libgl1-mesa-dri",
    "libegl-mesa0",
    "libgles2",
    "vulkan-tools",
];

pub fn plan(hw: &Hardware) -> Plan {
    let mut packages: Vec<String> = BASE_PACKAGES.iter().map(|s| s.to_string()).collect();
    let mut nvidia_packages = Vec::new();
    let mut modules = Vec::new();
    let mut reasons = Vec::new();
    let add = |list: &mut Vec<String>, items: &[&str]| {
        for i in items {
            if !list.iter().any(|x| x == i) {
                list.push(i.to_string());
            }
        }
    };
    for g in &hw.gpus {
        match g.vendor {
            GpuVendor::Intel => {
                add(
                    &mut packages,
                    &["firmware-intel-graphics", "firmware-misc-nonfree"],
                );
                let m = match g.driver.as_deref() {
                    Some("xe") => "xe",
                    _ => "i915",
                };
                add(&mut modules, &[m]);
                reasons.push(format!(
                    "{}: kernel driver {m}, Vulkan via Mesa ANV, Intel firmware",
                    g.name
                ));
            }
            GpuVendor::Amd => {
                add(&mut packages, &["firmware-amd-graphics"]);
                add(&mut modules, &["amdgpu"]);
                reasons.push(format!(
                    "{}: kernel driver amdgpu, Vulkan via Mesa RADV, AMD firmware",
                    g.name
                ));
            }
            GpuVendor::Nvidia => {
                add(
                    &mut nvidia_packages,
                    &[
                        "nvidia-driver",
                        "nvidia-vulkan-icd",
                        "firmware-nvidia-graphics",
                        "linux-headers-amd64",
                    ],
                );
                add(&mut modules, &["nvidia"]);
                reasons.push(format!(
                    "{}: proprietary driver from Debian non-free (chosen by nvidia-detect), \
                     NVIDIA Vulkan; only with your permission, never CUDA",
                    g.name
                ));
            }
            GpuVendor::Unknown => reasons.push(format!(
                "{}: no driver known for this GPU; only the generic Mesa packages",
                g.name
            )),
        }
    }
    let iommu_args = if hw.iommu.firmware && !hw.iommu.enabled {
        if hw.cpu.vendor.contains("AMD") {
            vec!["amd_iommu=on".into(), "iommu=pt".into()]
        } else {
            vec!["intel_iommu=on".into(), "iommu=pt".into()]
        }
    } else {
        Vec::new()
    };
    Plan {
        packages,
        nvidia_packages,
        modules,
        iommu_args,
        reasons,
    }
}

// ------------------------------------------------------------------ validação

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VkKind {
    Integrated,
    Discrete,
    Virtual,
    Cpu,
    Other,
}

/// Um device do `vulkaninfo --summary`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VkDevice {
    pub name: String,
    pub vendor_id: u16,
    pub device_id: u16,
    pub api: (u32, u32),
    pub kind: VkKind,
}

/// Lê os blocos `GPUn:` do `vulkaninfo --summary`.
pub fn parse_vulkan_summary(text: &str) -> Vec<VkDevice> {
    let mut out = Vec::new();
    let mut cur: Option<VkDevice> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("GPU") && t.ends_with(':') {
            if let Some(d) = cur.take() {
                out.push(d);
            }
            cur = Some(VkDevice {
                name: String::new(),
                vendor_id: 0,
                device_id: 0,
                api: (0, 0),
                kind: VkKind::Other,
            });
            continue;
        }
        let (Some(d), Some((k, v))) = (cur.as_mut(), t.split_once('=')) else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "apiVersion" => d.api = parse_api(v),
            "vendorID" => d.vendor_id = parse_hex16(v),
            "deviceID" => d.device_id = parse_hex16(v),
            "deviceName" => d.name = v.to_string(),
            "deviceType" => {
                d.kind = match v {
                    x if x.contains("INTEGRATED") => VkKind::Integrated,
                    x if x.contains("DISCRETE") => VkKind::Discrete,
                    x if x.contains("VIRTUAL") => VkKind::Virtual,
                    x if x.contains("CPU") => VkKind::Cpu,
                    _ => VkKind::Other,
                }
            }
            _ => {}
        }
    }
    if let Some(d) = cur {
        out.push(d);
    }
    out
}

/// `1.3.296` ou `4206847 (1.3.255)` → `(1, 3)`.
fn parse_api(v: &str) -> (u32, u32) {
    let s = v
        .split(['(', ')'])
        .find(|p| p.contains('.'))
        .unwrap_or(v)
        .trim();
    let mut it = s.split('.').map(|x| x.trim().parse::<u32>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0))
}

fn parse_hex16(v: &str) -> u16 {
    let v = v.trim();
    let v = v.strip_prefix("0x").unwrap_or(v);
    u16::from_str_radix(v, 16).unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    /// A GPU do monitor respondeu ao Vulkan.
    Ready,
    /// Só há renderização por software: o macOS fica lento demais.
    Software,
    /// A GPU não passou.
    Fail,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Ready => "READY",
            Verdict::Software => "SOFTWARE",
            Verdict::Fail => "FAIL",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "READY" => Some(Verdict::Ready),
            "SOFTWARE" => Some(Verdict::Software),
            "FAIL" => Some(Verdict::Fail),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuCheck {
    pub slot: String,
    pub name: String,
    pub display: bool,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validation {
    pub verdict: Verdict,
    /// Nome da GPU do monitor, como o Vulkan a mostra.
    pub display_name: String,
    pub gpus: Vec<GpuCheck>,
    pub kvm: bool,
    pub message: String,
}

/// Decide o veredito a partir do hardware e dos devices Vulkan.
pub fn validate_with(hw: &Hardware, vk: &[VkDevice]) -> Validation {
    let hardware_ok = |d: &&VkDevice| {
        !matches!(d.kind, VkKind::Cpu) && (d.api.0 > 1 || (d.api.0 == 1 && d.api.1 >= 2))
    };
    let mut checks = Vec::new();
    for g in &hw.gpus {
        let m = vk
            .iter()
            .find(|d| d.vendor_id == g.vendor_id && d.device_id == g.device_id);
        let (ok, detail) = match m {
            None => (
                false,
                format!(
                    "no Vulkan device (driver: {})",
                    g.driver.as_deref().unwrap_or("none")
                ),
            ),
            Some(d) if !hardware_ok(&d) => (
                false,
                format!("Vulkan {}.{} is below 1.2", d.api.0, d.api.1),
            ),
            Some(d) => (true, format!("Vulkan {}.{} · {}", d.api.0, d.api.1, d.name)),
        };
        checks.push(GpuCheck {
            slot: g.slot.clone(),
            name: m.map(|d| d.name.clone()).unwrap_or_else(|| g.name.clone()),
            display: g.display,
            ok,
            detail,
        });
    }
    let display = checks.iter().find(|c| c.display).or_else(|| checks.first());
    let software = vk.iter().any(|d| d.kind == VkKind::Cpu);
    let (verdict, display_name, message) = match display {
        Some(c) if c.ok => (
            Verdict::Ready,
            c.name.clone(),
            format!("Graphics: ready - {}", c.name),
        ),
        Some(c) if checks.iter().any(|x| x.ok) => (
            Verdict::Fail,
            c.name.clone(),
            format!(
                "the monitor's GPU ({}) does not answer to Vulkan; another GPU does: \
                 connect the monitor to it",
                c.name
            ),
        ),
        _ if software => (
            Verdict::Software,
            String::new(),
            "only software rendering (llvmpipe): macOS would be far too slow".into(),
        ),
        Some(c) => (
            Verdict::Fail,
            c.name.clone(),
            format!("{} did not pass: {}", c.name, c.detail),
        ),
        None => (Verdict::Fail, String::new(), "no GPU found".into()),
    };
    Validation {
        verdict,
        display_name,
        gpus: checks,
        kvm: hw.kvm,
        message,
    }
}

/// Roda `vulkaninfo --summary` e valida.
pub fn validate(hw: &Hardware) -> Validation {
    let text = crate::sys::output("vulkaninfo", &["--summary"]).unwrap_or_default();
    validate_with(hw, &parse_vulkan_summary(&text))
}

// ------------------------------------------------------------------ estado salvo

/// Onde o hardware lido no boot fica salvo (`reimsctl hw detect --save`).
pub fn saved_path() -> PathBuf {
    crate::brand::system_state_dir().join("hw.json")
}

/// Onde o veredito dos gráficos fica salvo pelo first setup.
pub fn verdict_path() -> PathBuf {
    crate::brand::system_state_dir().join("graphics")
}

pub fn saved_verdict() -> Option<Verdict> {
    fs::read_to_string(verdict_path())
        .ok()
        .and_then(|s| Verdict::parse(s.lines().next().unwrap_or("")))
}

/// Relatório legível (`reimsctl hw report`).
pub fn report(hw: &Hardware, plan: &Plan, v: &Validation) -> String {
    let yes = |b: bool| if b { "yes" } else { "no" };
    let mut s = String::new();
    s.push_str(&format!(
        "Processor      {} ({})\n",
        hw.cpu.model, hw.cpu.vendor
    ));
    s.push_str(&format!(
        "Virtualization {}\n",
        hw.cpu
            .virt
            .as_deref()
            .unwrap_or("none: enable VT-x/AMD-V in the firmware")
    ));
    s.push_str(&format!("KVM            {}\n", yes(hw.kvm)));
    s.push_str(&format!(
        "IOMMU          firmware: {}, enabled: {}\n",
        yes(hw.iommu.firmware),
        yes(hw.iommu.enabled)
    ));
    if hw.gpus.is_empty() {
        s.push_str("GPU            none found\n");
    }
    for (i, g) in hw.gpus.iter().enumerate() {
        s.push_str(&format!(
            "GPU{i}           {} [{}] driver: {}{}\n",
            g.name,
            g.id(),
            g.driver.as_deref().unwrap_or("none"),
            if g.display { " · monitor" } else { "" }
        ));
    }
    s.push_str("\nPlan\n");
    for r in &plan.reasons {
        s.push_str(&format!("  {r}\n"));
    }
    s.push_str(&format!("  packages: {}\n", plan.packages.join(" ")));
    if !plan.nvidia_packages.is_empty() {
        s.push_str(&format!(
            "  NVIDIA (opt-in): {}\n",
            plan.nvidia_packages.join(" ")
        ));
    }
    if !plan.iommu_args.is_empty() {
        s.push_str(&format!(
            "  IOMMU (optional): {}\n",
            plan.iommu_args.join(" ")
        ));
    }
    s.push_str(&format!("\nGraphics       {}\n", v.verdict.as_str()));
    for c in &v.gpus {
        s.push_str(&format!(
            "  {} {} — {}\n",
            if c.ok { "✓" } else { "✗" },
            c.name,
            c.detail
        ));
    }
    s.push_str(&format!("  {}\n", v.message));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("reimsctl-hw-{}-{n}", std::process::id()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn mkgpu(root: &Path, slot: &str, vendor: &str, device: &str, driver: &str, vga: bool) {
        let d = root.join("sys/bus/pci/devices").join(slot);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("class"), "0x030000\n").unwrap();
        fs::write(d.join("vendor"), format!("{vendor}\n")).unwrap();
        fs::write(d.join("device"), format!("{device}\n")).unwrap();
        fs::write(d.join("driver_name"), driver).unwrap();
        fs::write(d.join("boot_vga"), if vga { "1\n" } else { "0\n" }).unwrap();
    }

    fn fixture() -> PathBuf {
        let root = temp_root();
        fs::create_dir_all(root.join("proc")).unwrap();
        fs::write(
            root.join("proc/cpuinfo"),
            "vendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Core(TM) i9-14900K\nflags\t\t: fpu vmx sse2\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("dev")).unwrap();
        fs::write(root.join("dev/kvm"), "").unwrap();
        fs::create_dir_all(root.join("sys/firmware/acpi/tables")).unwrap();
        fs::write(root.join("sys/firmware/acpi/tables/DMAR"), "").unwrap();
        // nomes de diretório sem ':' (ilegal no Windows); o código troca '_' por ':'.
        mkgpu(&root, "0000_00_02.0", "0x8086", "0xa780", "i915", true);
        mkgpu(&root, "0000_03_00.0", "0x1002", "0x73ff", "amdgpu", false);
        let net = root.join("sys/bus/pci/devices/0000_04_00.0");
        fs::create_dir_all(&net).unwrap();
        fs::write(net.join("class"), "0x020000\n").unwrap();
        fs::write(net.join("vendor"), "0x10ec\n").unwrap();
        fs::write(net.join("device"), "0x8125\n").unwrap();
        root
    }

    const SUMMARY: &str = "\
Devices:
========
GPU0:
\tapiVersion         = 1.3.296
\tvendorID           = 0x8086
\tdeviceID           = 0xa780
\tdeviceType         = PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU
\tdeviceName         = Intel(R) Graphics (RPL-S)
GPU1:
\tapiVersion         = 4206847 (1.3.255)
\tvendorID           = 0x10005
\tdeviceID           = 0x0000
\tdeviceType         = PHYSICAL_DEVICE_TYPE_CPU
\tdeviceName         = llvmpipe (LLVM 19.1.7, 256 bits)
";

    #[test]
    fn detects_cpu_kvm_iommu_and_gpus() {
        let root = fixture();
        let hw = detect_from(&root);
        assert_eq!(hw.cpu.virt.as_deref(), Some("VT-x"));
        assert!(hw.kvm);
        assert!(hw.iommu.firmware && !hw.iommu.enabled);
        assert_eq!(hw.gpus.len(), 2, "the network card is not a GPU");
        assert_eq!(hw.display_gpu().unwrap().vendor, GpuVendor::Intel);
        assert_eq!(hw.gpus[1].driver.as_deref(), Some("amdgpu"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn plan_per_vendor_and_iommu() {
        let root = fixture();
        let hw = detect_from(&root);
        let p = plan(&hw);
        assert!(p.packages.contains(&"firmware-intel-graphics".to_string()));
        assert!(p.packages.contains(&"firmware-amd-graphics".to_string()));
        assert!(p.nvidia_packages.is_empty());
        assert_eq!(p.modules, vec!["i915", "amdgpu"]);
        assert_eq!(p.iommu_args, vec!["intel_iommu=on", "iommu=pt"]);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn parses_summary_both_api_formats() {
        let d = parse_vulkan_summary(SUMMARY);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].api, (1, 3));
        assert_eq!(d[0].kind, VkKind::Integrated);
        assert_eq!(d[1].api, (1, 3));
        assert_eq!(d[1].kind, VkKind::Cpu);
    }

    #[test]
    fn ready_when_monitor_gpu_answers() {
        let root = fixture();
        let hw = detect_from(&root);
        let v = validate_with(&hw, &parse_vulkan_summary(SUMMARY));
        assert_eq!(v.verdict, Verdict::Ready);
        assert_eq!(v.message, "Graphics: ready - Intel(R) Graphics (RPL-S)");
        assert!(!v.gpus[1].ok, "the AMD card has no Vulkan device here");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn software_when_only_llvmpipe() {
        let root = fixture();
        let hw = detect_from(&root);
        let only_cpu: Vec<_> = parse_vulkan_summary(SUMMARY)
            .into_iter()
            .filter(|d| d.kind == VkKind::Cpu)
            .collect();
        assert_eq!(validate_with(&hw, &only_cpu).verdict, Verdict::Software);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn fail_when_monitor_gpu_missing_but_other_ok() {
        let root = fixture();
        let hw = detect_from(&root);
        let amd = VkDevice {
            name: "AMD Radeon RX 6600 (RADV NAVI23)".into(),
            vendor_id: 0x1002,
            device_id: 0x73ff,
            api: (1, 4),
            kind: VkKind::Discrete,
        };
        let v = validate_with(&hw, &[amd]);
        assert_eq!(v.verdict, Verdict::Fail);
        assert!(v.message.contains("monitor"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn verdict_roundtrip() {
        for v in [Verdict::Ready, Verdict::Software, Verdict::Fail] {
            assert_eq!(Verdict::parse(v.as_str()), Some(v));
        }
    }
}
