//! Execução da VM: criação do disco qcow2 e montagem do processo QEMU.
//!
//! Estas funções invocam os binários `qemu-img`/`qemu-system-x86_64` do host
//! (presentes no appliance a partir da Fase 2). São propositalmente finas — a
//! lógica de argumentos vive em [`crate::qemu`].

use crate::qemu;
use crate::vm::VmConfig;
use anyhow::{ensure, Context, Result};
use std::process::Command;

pub const QEMU_BIN: &str = "qemu-system-x86_64";
pub const QEMU_IMG: &str = "qemu-img";

/// Cria o disco qcow2 da VM, se ainda não existir.
pub fn create_disk(vm: &VmConfig) -> Result<()> {
    let disk = vm.disk_path();
    if disk.exists() {
        return Ok(());
    }
    let status = Command::new(QEMU_IMG)
        .arg("create")
        .arg("-f")
        .arg("qcow2")
        .arg(&disk)
        .arg(format!("{}G", vm.disk_gb))
        .status()
        .with_context(|| format!("executando {QEMU_IMG}"))?;
    ensure!(status.success(), "qemu-img create falhou ({status})");
    Ok(())
}

/// Monta o [`Command`] do QEMU para a VM (sem executar).
pub fn qemu_command(vm: &VmConfig) -> Result<Command> {
    let args = qemu::build_args(vm)?;
    let mut cmd = Command::new(QEMU_BIN);
    cmd.args(args);
    Ok(cmd)
}

/// Sobe a VM (bloqueante até o QEMU sair).
pub fn start(vm: &VmConfig) -> Result<()> {
    let status = qemu_command(vm)?
        .status()
        .with_context(|| format!("executando {QEMU_BIN}"))?;
    ensure!(status.success(), "QEMU saiu com erro ({status})");
    Ok(())
}
