//! reimsctl — binário do gerenciador de VMs do OSX-REIMS-OS.
//!
//! Uso (fase-0/1, mínimo):
//!   reimsctl menu                    → mostra o menu principal
//!   reimsctl releases                → lista versões de macOS
//!   reimsctl identity                → gera uma identidade de VM (JSON)
//!   reimsctl qemu-args <vm.json>     → imprime a linha do QEMU para uma VM
//!   reimsctl update-check            → checa atualizações
//!   reimsctl vm list                 → lista VMs
//!   reimsctl vm create <nome> <rel>  → cria uma VM (rel: ventura|sonoma|sequoia|tahoe)
//!   reimsctl vm start <nome>         → sobe uma VM no QEMU

use anyhow::{Context, Result};
use reimsctl_core::{launch, qemu, release::MacosRelease, vm::VmConfig, Store};

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
        "vm" => {
            vm_subcommand(&args)?;
        }
        other => {
            eprintln!("comando desconhecido: {other}");
            eprintln!(
                "comandos: menu | releases | identity | qemu-args <vm.json> | update-check | vm ..."
            );
            std::process::exit(2);
        }
    }
    Ok(())
}

fn vm_subcommand(args: &[String]) -> Result<()> {
    let store = Store::new(Store::default_root());
    match args.get(1).map(String::as_str) {
        Some("list") => {
            let names = store.list()?;
            if names.is_empty() {
                println!("(nenhuma VM)");
            }
            for name in names {
                println!("{name}");
            }
        }
        Some("create") => {
            let name = args
                .get(2)
                .context("uso: reimsctl vm create <nome> <release>")?;
            let rel = args
                .get(3)
                .context("uso: reimsctl vm create <nome> <release>")?;
            let macos = MacosRelease::parse(rel).with_context(|| {
                format!("release inválida: {rel} (use ventura|sonoma|sequoia|tahoe)")
            })?;
            let cfg = store.create(name, macos)?;
            println!(
                "criada VM '{}' (macOS {}) em {}",
                cfg.name,
                cfg.macos.code_name(),
                cfg.dir.display()
            );
        }
        Some("start") => {
            let name = args.get(2).context("uso: reimsctl vm start <nome>")?;
            let cfg = store.load(name)?;
            launch::create_disk(&cfg)?;
            launch::start(&cfg)?;
        }
        _ => {
            eprintln!("uso: reimsctl vm <list|create <nome> <release>|start <nome>>");
            std::process::exit(2);
        }
    }
    Ok(())
}
