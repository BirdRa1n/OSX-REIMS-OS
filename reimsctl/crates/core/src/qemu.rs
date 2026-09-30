//! Montagem da linha de comando do QEMU para uma VM macOS acelerada.
//!
//! Baseado no padrão do ecossistema OSX-KVM (q35 + OVMF + OpenCore + AppleSMC),
//! com o device `reims-vgpu-pci` no lugar do framebuffer básico.
//!
//! ⚠️ Esta é a base a ser **validada na Fase 0** (ver `docs/ROADMAP.md`); os
//! valores exatos (flags de CPU, ordem de devices, keyframe/vgpu) serão fixados
//! em `docs/QEMU-LINE.md` após o PoC.

use crate::vm::VmConfig;
use anyhow::{bail, Result};
use std::path::Path;

/// Caminho do firmware OVMF *read-only* (código). Ajustável por host.
pub const OVMF_CODE: &str = "/usr/share/OVMF/OVMF_CODE.fd";

/// Monta os argumentos do QEMU para dar boot na VM.
///
/// `qemu_bin` é o binário (ex.: `qemu-system-x86_64`); o retorno **não** o inclui.
pub fn build_args(vm: &VmConfig) -> Result<Vec<String>> {
    if vm.applesmc_osk.trim().is_empty() {
        bail!("applesmc_osk vazio: o macOS não sobe sem a chave OSK do AppleSMC (fornecida pelo usuário)");
    }
    if !Path::new(OVMF_CODE).exists() {
        // não é fatal em dev; a Fase 2 garante o pacote `ovmf`.
        eprintln!("aviso: OVMF_CODE não encontrado em {OVMF_CODE}");
    }

    let mut a: Vec<String> = Vec::new();
    let mut push = |parts: &[&str]| a.extend(parts.iter().map(|s| s.to_string()));

    push(&["-enable-kvm"]);
    push(&["-machine", "q35"]);
    push(&["-m", &vm.ram_mb.to_string()]);
    push(&["-smp", &format!("cores={},sockets=1", vm.cpus)]);

    // CPU: macOS exige flags específicas; host-passthrough + invtsc.
    // TODO(fase-0): ajustar vendor e flags por microarquitetura.
    push(&[
        "-cpu",
        "host,vendor=GenuineIntel,+invtsc,+hypervisor,kvm=on,vmware-cpuid-freq=on",
    ]);

    // AppleSMC — OSK fornecida pelo usuário (não redistribuída por nós).
    push(&["-device", &format!("isa-applesmc,osk={}", vm.applesmc_osk)]);

    // Firmware: OVMF (código somente-leitura + vars por-VM graváveis).
    push(&[
        "-drive",
        &format!("if=pflash,format=raw,readonly=on,file={OVMF_CODE}"),
    ]);
    push(&[
        "-drive",
        &format!(
            "if=pflash,format=raw,file={}",
            vm.ovmf_vars_path().display()
        ),
    ]);

    // OpenCore (bootloader) + disco do macOS.
    push(&[
        "-device",
        "ide-hd,bus=sata.2,drive=OpenCoreBoot",
        "-drive",
        &format!(
            "id=OpenCoreBoot,if=none,format=qcow2,file={}",
            vm.opencore_path().display()
        ),
    ]);
    push(&[
        "-device",
        "ide-hd,bus=sata.3,drive=MacHDD",
        "-drive",
        &format!(
            "id=MacHDD,if=none,format=qcow2,file={}",
            vm.disk_path().display()
        ),
    ]);
    push(&["-device", "ich9-ahci,id=sata"]);

    // GPU acelerada via reims-vgpu, ou framebuffer básico como fallback.
    if vm.vgpu {
        // TODO(fase-0): confirmar nome/props exatos do device do reims-vgpu.
        push(&["-device", "reims-vgpu-pci"]);
    } else {
        push(&["-device", "VGA"]);
    }

    // Entrada + rede.
    push(&["-usb", "-device", "usb-tablet", "-device", "usb-kbd"]);
    push(&["-netdev", "user,id=net0", "-device", "vmxnet3,netdev=net0"]);

    // Firmware SMBIOS mínimo; identidade real vem do OpenCore (crate `efi`).
    push(&["-smbios", "type=2"]);

    Ok(a)
}
