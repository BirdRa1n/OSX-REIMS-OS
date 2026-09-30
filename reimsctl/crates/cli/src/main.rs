//! reimsctl — binário do gerenciador de VMs do OSX-REIMS-OS.
//!
//! Uso (fase-0/1, mínimo):
//!   reimsctl menu                    → mostra o menu principal
//!   reimsctl releases                → lista versões de macOS
//!   reimsctl identity                → gera uma identidade de VM (JSON)
//!   reimsctl identity-inject <plist> → gera identidade e injeta no config.plist
//!   reimsctl gpu-detect              → detecta a GPU do host (intel|amd|nvidia|unknown)
//!   reimsctl fetch-macos <rel> <dir> → baixa o Recovery do macOS da Apple
//!   reimsctl qemu-args <vm.json>     → imprime a linha do QEMU para uma VM
//!   reimsctl update-check            → checa atualizações
//!   reimsctl vm list                 → lista VMs
//!   reimsctl vm create <nome> <rel>  → cria uma VM (rel: ventura|sonoma|sequoia|tahoe)
//!   reimsctl vm start <nome>         → sobe uma VM no QEMU
//!   reimsctl vm provision <nome> <rel> → cria + baixa macOS + OpenCore + disco

use anyhow::{Context, Result};
use reimsctl_core::{launch, paths, qemu, release::MacosRelease, vm::VmConfig, Store};

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
        "identity-inject" => {
            let path = args
                .get(1)
                .context("uso: reimsctl identity-inject <config.plist>")?;
            let id = reimsctl_efi::Identity::generate();
            reimsctl_efi::set_platform_identity(std::path::Path::new(path), &id)?;
            eprintln!("identidade injetada em {path}");
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
        "gpu-detect" => {
            let v = reimsctl_core::gpu::detect_primary();
            println!("{}", v.as_str());
            eprintln!(
                "driver Vulkan: {} | opt-in: {}",
                v.vulkan_driver(),
                v.requires_optin()
            );
        }
        "fetch-macos" => {
            let rel = args
                .get(1)
                .context("uso: reimsctl fetch-macos <release> <dir>")?;
            let dir = args
                .get(2)
                .context("uso: reimsctl fetch-macos <release> <dir>")?;
            let macos =
                MacosRelease::parse(rel).with_context(|| format!("release inválida: {rel}"))?;
            let img = reimsctl_macos::fetch_recovery(macos, std::path::Path::new(dir))?;
            println!("baixado: {}", img.path.display());
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
                "comandos: menu | releases | identity | identity-inject <plist> | gpu-detect | fetch-macos <rel> <dir> | qemu-args <vm.json> | update-check | vm ..."
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
        Some("provision") => {
            let name = args
                .get(2)
                .context("uso: reimsctl vm provision <nome> <release>")?;
            let rel = args
                .get(3)
                .context("uso: reimsctl vm provision <nome> <release>")?;
            let macos =
                MacosRelease::parse(rel).with_context(|| format!("release inválida: {rel}"))?;
            vm_provision(&store, name, macos)?;
        }
        _ => {
            eprintln!(
                "uso: reimsctl vm <list|create <nome> <rel>|start <nome>|provision <nome> <rel>>"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}

/// Fluxo end-to-end: cria a VM, baixa o macOS, prepara o OpenCore com identidade
/// única e cria o disco. Deixa pronta para `vm start` (falta só a OSK).
fn vm_provision(store: &Store, name: &str, macos: MacosRelease) -> Result<()> {
    println!(">> [1/4] criando VM '{name}' (macOS {})", macos.code_name());
    let cfg = store.create(name, macos)?;

    println!(">> [2/4] baixando macOS Recovery da Apple (pode demorar)");
    reimsctl_macos::fetch_recovery(macos, &cfg.dir)?;

    println!(">> [3/4] preparando OpenCore + identidade única");
    let base = paths::opencore_base_image(&paths::opencore_base_dir(), macos);
    anyhow::ensure!(
        base.is_file(),
        "base do OpenCore ausente: {} (rode opencore/fetch-base.sh e instale em {})",
        base.display(),
        paths::DEFAULT_OPENCORE_BASE_DIR
    );
    std::fs::copy(&base, cfg.opencore_path()).with_context(|| {
        format!(
            "copiando {} -> {}",
            base.display(),
            cfg.opencore_path().display()
        )
    })?;

    let id = reimsctl_efi::Identity::generate();
    let plist = cfg.dir.join("config.plist");
    let oc_img = cfg.opencore_path();
    run_oc_image("extract", &oc_img, &plist)?;
    reimsctl_efi::set_platform_identity(&plist, &id)?;
    run_oc_image("inject", &oc_img, &plist)?;
    std::fs::write(
        cfg.dir.join("identity.json"),
        serde_json::to_string_pretty(&id)?,
    )?;

    println!(">> [4/4] criando disco da VM");
    launch::create_disk(&cfg)?;

    println!(
        "VM '{name}' pronta. Defina 'applesmc_osk' em {} e rode: reimsctl vm start {name}",
        cfg.config_path().display()
    );
    Ok(())
}

/// Invoca o `oc-image.sh` (`extract`/`inject`) via bash.
fn run_oc_image(action: &str, img: &std::path::Path, plist: &std::path::Path) -> Result<()> {
    let script = paths::oc_image_script();
    let status = std::process::Command::new("bash")
        .arg(&script)
        .arg(action)
        .arg(img)
        .arg(plist)
        .status()
        .with_context(|| format!("executando {}", script.display()))?;
    anyhow::ensure!(status.success(), "oc-image.sh {action} falhou ({status})");
    Ok(())
}
