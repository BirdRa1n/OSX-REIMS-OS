//! reimsctl — binário do gerenciador de VMs do OSX-REIMS-OS.
//!
//! Uso (fase-0, mínimo):
//!   reimsctl menu                 → mostra o menu principal
//!   reimsctl releases             → lista versões de macOS
//!   reimsctl identity             → gera uma identidade de VM (JSON)
//!   reimsctl qemu-args <vm.json>  → imprime a linha do QEMU para uma VM
//!   reimsctl update-check         → checa atualizações

use anyhow::{Context, Result};
use reimsctl_core::{qemu, vm::VmConfig};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("menu");

    match cmd {
        "menu" => {
            reimsctl_tui::render_main_menu()?;
        }
        "releases" => {
            reimsctl_tui::print_releases();
        }
        "identity" => {
            let id = reimsctl_efi::Identity::generate();
            println!("{}", serde_json::to_string_pretty(&id)?);
        }
        "qemu-args" => {
            let path = args.get(1).context("uso: reimsctl qemu-args <vm.json>")?;
            let data = std::fs::read_to_string(path).with_context(|| format!("lendo {path}"))?;
            let vm: VmConfig =
                serde_json::from_str(&data).with_context(|| format!("parseando {path}"))?;
            let qemu_args = qemu::build_args(&vm)?;
            println!("qemu-system-x86_64 {}", qemu_args.join(" "));
        }
        "update-check" => {
            let st = reimsctl_updater::check()?;
            match st.latest {
                Some(v) => println!("atual {} → disponível {}", st.current, v),
                None => println!("atual {} (sem atualização)", st.current),
            }
        }
        other => {
            eprintln!("comando desconhecido: {other}");
            eprintln!("comandos: menu | releases | identity | qemu-args <vm.json> | update-check");
            std::process::exit(2);
        }
    }
    Ok(())
}
