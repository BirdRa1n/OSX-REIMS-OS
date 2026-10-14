//! reimsctl-tui — o console do appliance: criar, importar, gerenciar e manter as
//! VMs macOS, ver logs e preparar o ambiente.
//!
//! É um console de texto de tecla única. Na sessão do appliance ele roda em
//! tela cheia, num terminal dentro do compositor Wayland (cage); as janelas das
//! VMs abrem no mesmo compositor, por cima dele.

mod term;

use anyhow::{bail, Context, Result};
use reimsctl_core::{brand, ctl, hw, state, MacosRelease, Verdict};
use std::path::{Path, PathBuf};
use std::process::Command;
use term::*;

pub const LICENSE_TEXT: &str = include_str!("license.txt");

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Inicia a VM marcada para início automático (o primeiro console da sessão).
    pub autostart: bool,
}

/// Por que criar/importar/gerenciar ainda não estão disponíveis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Blocker {
    FirstSetup,
    Graphics(String),
    License,
    Prepare(String),
}

impl Blocker {
    fn text(&self) -> String {
        match self {
            Blocker::FirstSetup => {
                "The first setup has not finished: run sudo reims-firstboot --force".into()
            }
            Blocker::Graphics(m) => {
                format!("The graphics did not validate ({m}). See: reimsctl hw validate")
            }
            Blocker::License => "The license has not been accepted on this computer yet.".into(),
            Blocker::Prepare(r) => r.clone(),
        }
    }
}

/// `true` dentro da imagem instalada (os scripts do appliance existem).
fn appliance() -> bool {
    Path::new("/usr/lib/reims/brand.sh").exists()
}

pub fn blocker() -> Option<Blocker> {
    if appliance() && !brand::system_state_dir().join("firstboot.done").exists() {
        return Some(Blocker::FirstSetup);
    }
    if hw::saved_verdict() == Some(Verdict::Fail) {
        return Some(Blocker::Graphics("FAIL".into()));
    }
    if !state::license_accepted() {
        return Some(Blocker::License);
    }
    match state::prepared() {
        None => Some(Blocker::Prepare(
            "The environment is not prepared yet: Reims vGPU has to be downloaded and built."
                .into(),
        )),
        Some(p) if p.version != brand::VERSION => Some(Blocker::Prepare(format!(
            "{} was updated ({} → {}): prepare the environment again.",
            brand::PRODUCT,
            p.version,
            brand::VERSION
        ))),
        Some(_) if !ctl::installed() => Some(Blocker::Prepare(
            "reims-ctl is missing: prepare the environment again.".into(),
        )),
        Some(_) => None,
    }
}

// ------------------------------------------------------------------ tela principal

fn header(title: &str) {
    clear();
    accent(brand::PRODUCT);
    print!(" · macOS virtual machines · v{}", brand::VERSION);
    if !title.is_empty() {
        print!(" · ");
        bold(title);
    }
    println!();
    rule();
}

fn footer() {
    rule();
    dim("GPU virtualization by Reims vGPU · https://github.com/steelbrain/reims-vgpu\n");
}

/// Uma VM como a lista mostra.
struct Row {
    entry: ctl::VmEntry,
    meta: state::VmMeta,
    installer: bool,
}

impl Row {
    fn id(&self) -> u32 {
        self.entry.id().unwrap_or(0)
    }

    fn state(&self) -> String {
        let s = &self.entry.state;
        if s == "running" {
            return "Running".into();
        }
        if s == "broken" {
            return "Faulty (see View logs)".into();
        }
        match (
            self.installer,
            self.meta.first_start_done,
            self.meta.source.as_str(),
        ) {
            (true, false, _) => "Ready to install".into(),
            (true, true, _) => "Installer attached (eject it once macOS is installed)".into(),
            (false, false, "import") => "Ready for first use".into(),
            _ => "Installed".into(),
        }
    }

    fn installed(&self) -> bool {
        !self.installer && self.meta.first_start_done
    }

    fn macos(&self) -> String {
        MacosRelease::parse(&self.meta.macos)
            .map(|m| format!("macOS {} ({})", m.code_name(), m.major()))
            .unwrap_or_else(|| "macOS".into())
    }
}

fn rows() -> Vec<Row> {
    if !ctl::installed() {
        return Vec::new();
    }
    let metas = state::vms();
    ctl::list()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.id().is_some())
        .map(|entry| {
            let installer = ctl::show(&entry.name)
                .map(|kv| kv.iter().any(|(k, v)| k == "installer" && v != "none"))
                .unwrap_or(false);
            Row {
                meta: metas.get(&entry.name).cloned().unwrap_or_default(),
                entry,
                installer,
            }
        })
        .collect()
}

fn print_rows(rows: &[Row]) {
    bold("VMs\n");
    if rows.is_empty() {
        dim("  (no VMs created)\n");
    }
    let auto = state::autostart();
    for r in rows {
        print!("  ");
        accent(&format!("{:>2}", r.id()));
        print!("  {:<22} ", r.macos());
        if r.entry.running() {
            good(&r.state());
        } else {
            print!("{}", r.state());
        }
        if auto.as_deref() == Some(r.entry.name.as_str()) {
            dim("  · starts with the computer");
        }
        println!();
    }
    println!();
}

/// O console. Volta quando o usuário escolhe Quit.
pub fn run(opts: Options) -> Result<()> {
    if opts.autostart {
        autostart_now();
    }
    loop {
        let block = blocker();
        let ready = block.is_none();
        header("");
        let rows = rows();
        print_rows(&rows);
        if let Some(b) = &block {
            warn(&format!("{}\n", b.text()));
            if matches!(b, Blocker::License | Blocker::Prepare(_)) {
                item(
                    "S",
                    "Prepare environment",
                    "license, packages, Rust, Reims vGPU build",
                    true,
                );
            }
            println!();
        }
        item("1", "Create new VM", "", ready);
        item(
            "2",
            "Import VM disk",
            "QCOW2 of a macOS already installed in another VM",
            ready,
        );
        item("3", "Manage VMs", "start, shut down, restart", ready);
        item("4", "View logs", "", true);
        item(
            "5",
            "Maintenance",
            "delete VMs, clean cache, check this computer, Reims vGPU",
            true,
        );
        item("Q", "Quit", "", true);
        println!();
        footer();
        let k = key("Choose an option: ");
        let res = match k {
            '1' if ready => create_vm(),
            '2' if ready => import_vm(),
            '3' if ready => manage_vms(),
            '4' => view_logs(),
            '5' => maintenance(),
            's' if matches!(block, Some(Blocker::License | Blocker::Prepare(_))) => {
                prepare(&[]).map(|_| ())
            }
            'q' | '\x1b' | '\x03' => {
                // a janela de uma VM vive no mesmo compositor que este console.
                if !rows.iter().any(|r| r.entry.running())
                    || confirm(
                        "\nA VM is running, and quitting the console closes its window. Quit anyway?",
                        false,
                    )
                {
                    return Ok(());
                }
                Ok(())
            }
            _ => Ok(()),
        };
        if let Err(e) = res {
            bad(&format!("\nerror: {e:#}\n"));
            pause();
        }
    }
}

// ------------------------------------------------------------------ licença e preparo

fn show_license() -> Result<bool> {
    header("License");
    println!("{LICENSE_TEXT}");
    rule();
    let a = ask(
        "Type accept to accept the license, or press Enter to go back",
        "",
    );
    if a.eq_ignore_ascii_case("accept") {
        state::accept_license()?;
        good("✓ License accepted\n");
        Ok(true)
    } else {
        Ok(false)
    }
}

fn prepare_bin() -> PathBuf {
    std::env::var_os("REIMS_PREPARE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("reims-prepare"))
}

/// Roda o `reims-prepare` (as 10 etapas), com um log da ação.
pub fn prepare(args: &[&str]) -> Result<bool> {
    if !state::license_accepted() && !show_license()? {
        return Ok(false);
    }
    header("Prepare environment");
    let action = if args.contains(&"--update") {
        "update-reims"
    } else if args.contains(&"--rebuild") {
        "rebuild-reims"
    } else {
        "prepare"
    };
    let log = state::ActionLog::start(action, None)?;
    let status = Command::new(prepare_bin())
        .args(args)
        .arg("--log")
        .arg(&log.path)
        .status()
        .with_context(|| format!("running {}", prepare_bin().display()))?;
    let ok = status.success();
    log.finish(if ok { "Done" } else { "Failed" });
    pause();
    Ok(ok)
}

// ------------------------------------------------------------------ criar

fn used_ids() -> Vec<u32> {
    ctl::list()
        .unwrap_or_default()
        .iter()
        .filter_map(|e| e.id())
        .collect()
}

fn ask_id() -> Result<u32> {
    let used = used_ids();
    let free = ctl::first_free(&used).context("all VM numbers (10 to 99) are taken")?;
    loop {
        let s = ask("VM number (10 to 99)", &free.to_string());
        match ctl::parse_id(&s) {
            Some(i) if used.contains(&i) => bad(&format!("VM {i} already exists\n")),
            Some(i) => return Ok(i),
            None => bad("a number from 10 to 99\n"),
        }
    }
}

fn cached(m: MacosRelease) -> bool {
    let dir = brand::home().join(".local/share/reims-ctl/installers");
    std::fs::read_dir(dir)
        .map(|d| {
            d.flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(m.slug())
            })
        })
        .unwrap_or(false)
}

fn ask_macos() -> MacosRelease {
    println!("\nmacOS version:");
    for (i, m) in MacosRelease::ALL.iter().enumerate() {
        let note = if cached(*m) { "already downloaded" } else { "" };
        item(
            &(i + 1).to_string(),
            &format!("{} ({})", m.code_name(), m.major()),
            note,
            true,
        );
    }
    loop {
        let s = ask("Choose", "1");
        if let Some(m) = s
            .parse::<usize>()
            .ok()
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| MacosRelease::ALL.get(i))
            .copied()
            .or_else(|| MacosRelease::parse(&s))
        {
            return m;
        }
    }
}

fn full_images() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(brand::full_installers_dir())
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().is_some_and(|x| {
                        matches!(
                            x.to_string_lossy().to_ascii_lowercase().as_str(),
                            "iso" | "img" | "cdr" | "dmg"
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn ask_disk_size() -> String {
    loop {
        let s = ask("macOS disk size, such as 256G or 1T", "256G");
        let ok = s.len() > 1
            && s[..s.len() - 1].parse::<u64>().is_ok()
            && matches!(s.chars().last(), Some('G' | 'g' | 'T' | 't'));
        if ok {
            return s.to_uppercase();
        }
        bad("a size such as 256G or 1T\n");
    }
}

fn create_vm() -> Result<()> {
    header("Create new VM");
    let id = ask_id()?;
    println!("\nmacOS installer:");
    item(
        "1",
        "Recovery",
        "downloaded from Apple; installing macOS uses the internet",
        true,
    );
    item(
        "2",
        "Offline",
        "one full release from Apple, 13 to 20 GB; installs with no network",
        true,
    );
    item(
        "3",
        "Full",
        "an installer image you already have (createinstallmedia)",
        true,
    );
    let kind = loop {
        match ask("Choose", "1").as_str() {
            "1" => break "recovery",
            "2" => break "offline",
            "3" => break "full",
            _ => {}
        }
    };
    let mut macos = None;
    let mut image = None;
    if kind == "full" {
        let imgs = full_images();
        println!();
        dim(&format!(
            "Images in {} are listed here; you can also type a path.\n",
            brand::full_installers_dir().display()
        ));
        for (i, p) in imgs.iter().enumerate() {
            item(&(i + 1).to_string(), &p.display().to_string(), "", true);
        }
        let s = ask(
            "Image (number or path)",
            if imgs.is_empty() { "" } else { "1" },
        );
        let p = s
            .parse::<usize>()
            .ok()
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| imgs.get(i).cloned())
            .unwrap_or_else(|| PathBuf::from(&s));
        if !p.is_file() {
            bail!("{} does not exist", p.display());
        }
        image = Some(p);
    } else {
        macos = Some(ask_macos());
    }
    println!();
    let disk = ask_disk_size();
    let name = ctl::vm_name(id);

    println!();
    bold("Summary\n");
    println!("  VM          {id} ({name})");
    match (&macos, &image) {
        (Some(m), _) => println!("  macOS       {} ({}) · {kind}", m.code_name(), m.major()),
        (_, Some(p)) => println!("  installer   {}", p.display()),
        _ => {}
    }
    println!("  disk        {disk} (grows as it fills)");
    println!("  SSH         localhost:{}", ctl::ssh_port(id));
    println!();
    if !confirm("Create the VM?", true) {
        return Ok(());
    }

    let mut log = state::ActionLog::start("create", Some(&name))?;
    let port = ctl::ssh_port(id).to_string();
    let mut args: Vec<String> = vec!["create".into(), name.clone()];
    match (&macos, &image) {
        (Some(m), _) => {
            args.push("--macos".into());
            args.push(m.slug().into());
            if kind == "offline" {
                args.push("--offline".into());
            }
        }
        (_, Some(p)) => {
            args.push("--installer".into());
            args.push(p.display().to_string());
        }
        _ => {}
    }
    args.extend(
        [
            "--disk-size",
            &disk,
            "--borderless",
            "true",
            "--resolution",
            "host",
            "--ssh-port",
            &port,
            // a sessão kiosk não tem servidor de som.
            "--audio",
            "false",
        ]
        .map(String::from),
    );
    log.line(&format!("reims-ctl {}", args.join(" ")));
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    if let Err(e) = ctl::run_ok(&argv) {
        log.line(&format!("{e:#}"));
        log.finish("Failed");
        return Err(e);
    }
    state::save_vm(
        &name,
        &state::VmMeta {
            macos: macos
                .map(|m| m.slug().to_string())
                .unwrap_or_else(|| "other".into()),
            source: kind.into(),
            first_start_done: false,
        },
    )?;
    log.finish("Done");
    good(&format!("\n✓ VM {id} created.\n"));
    if key("Press S to start it now, any other key to go back. ") == 's' {
        start_vm(&name)?;
    }
    Ok(())
}

fn import_vm() -> Result<()> {
    header("Import VM disk");
    dim(
        "The disk is copied into the VM: the original file is not changed. The VM gets a\n\
         new Mac identity unless you also give the OpenCore image it booted with.\n\n",
    );
    let disk = PathBuf::from(ask("QCOW2 disk to import", ""));
    if !disk.is_file() {
        bail!("{} does not exist", disk.display());
    }
    let oc = ask("Its OpenCore image (Enter for a new one)", "");
    let id = ask_id()?;
    let label = ask(
        "macOS version shown in the list (ventura, sonoma, sequoia, tahoe, other)",
        "other",
    );
    let name = ctl::vm_name(id);
    if !confirm("Import the disk?", true) {
        return Ok(());
    }
    let mut log = state::ActionLog::start("import", Some(&name))?;
    let port = ctl::ssh_port(id).to_string();
    let disk_s = disk.display().to_string();
    let mut args = vec![
        "create",
        &name,
        "--disk",
        &disk_s,
        "--borderless",
        "true",
        "--resolution",
        "host",
        "--ssh-port",
        &port,
        "--audio",
        "false",
    ];
    if !oc.is_empty() {
        args.push("--opencore");
        args.push(&oc);
    }
    log.line(&format!("reims-ctl {}", args.join(" ")));
    if let Err(e) = ctl::run_ok(&args) {
        log.finish("Failed");
        return Err(e);
    }
    state::save_vm(
        &name,
        &state::VmMeta {
            macos: MacosRelease::parse(&label)
                .map(|m| m.slug().to_string())
                .unwrap_or_else(|| "other".into()),
            source: "import".into(),
            first_start_done: false,
        },
    )?;
    log.finish("Done");
    good(&format!("\n✓ VM {id} imported.\n"));
    if key("Press S to start it now, any other key to go back. ") == 's' {
        start_vm(&name)?;
    }
    Ok(())
}

// ------------------------------------------------------------------ iniciar

fn print_install_steps() {
    bold("Installing macOS\n");
    println!(
        "  1. In the OpenCore picker, choose the installer (macOS Recovery or Install macOS)."
    );
    println!("  2. Open Disk Utility and erase the VM's disk (QEMU HARDDISK Media) as APFS.");
    println!("  3. Install macOS on it; the VM restarts a few times.");
    println!("  4. Once macOS is installed, in the OpenCore picker choose the Recovery entry,");
    println!("     open Utilities › Terminal and run, once (without it the desktop never");
    println!("     shows: no menu bar, no Dock):");
    dim("       cd /Volumes/*Data/Library/Preferences\n");
    dim("       defaults write \"$PWD/.GlobalPreferences\" METAL_ERROR_MODE -int 5\n");
    dim("       chmod 644 .GlobalPreferences.plist\n");
    dim("       reboot\n");
    println!("  5. Back here, Manage VMs › C › E ejects the installer.");
    println!();
}

fn summary(name: &str) {
    if let Ok(kv) = ctl::show(name) {
        for (k, v) in kv {
            if matches!(
                k.as_str(),
                "state" | "cpus" | "memory" | "displays" | "gpu" | "installer" | "disk"
            ) {
                println!("  {k:<11} {v}");
            }
        }
    }
    if let Some(id) = ctl::parse_id(name) {
        println!(
            "  {:<11} localhost:{} (turn on Remote Login in macOS)",
            "SSH",
            ctl::ssh_port(id)
        );
    }
    if state::autostart().as_deref() == Some(name) {
        println!("  {:<11} starts with the computer", "autostart");
    }
}

fn start_vm(name: &str) -> Result<()> {
    let mut meta = state::vm(name).unwrap_or_default();
    header(&format!("Start VM {}", ctl::parse_id(name).unwrap_or(0)));
    if !meta.first_start_done {
        bold("First start: these answers are saved in the VM.\n\n");
        let mem = ask("Memory, such as 8G or 16G", "8G").to_uppercase();
        let cpus = loop {
            let s = ask("Processors (1 to 32)", "4");
            match s.parse::<u32>() {
                Ok(n) if (1..=32).contains(&n) => break n.to_string(),
                _ => bad("a number from 1 to 32\n"),
            }
        };
        ctl::run_ok(&["set", name, "--memory", &mem, "--cpus", &cpus])?;
        println!();
        if meta.source != "import" {
            print_install_steps();
        }
    }
    bold("Summary\n");
    summary(name);
    println!();
    loop {
        match key("S Start · C Change a setting · Q Cancel: ") {
            's' | '\n' => break,
            'c' => {
                change_settings(name)?;
                header(&format!("Start VM {}", ctl::parse_id(name).unwrap_or(0)));
                summary(name);
                println!();
            }
            'q' | '\x1b' | '\x03' => return Ok(()),
            _ => {}
        }
    }
    let mut log = state::ActionLog::start("start", Some(name))?;
    match ctl::run_ok(&["start", name]) {
        Ok(()) => {
            meta.first_start_done = true;
            state::save_vm(name, &meta)?;
            log.finish("Done");
            good("\n✓ The VM is starting in full screen.\n");
            dim("Shut macOS down (Apple menu › Shut Down) to come back to this console.\n");
            pause();
            Ok(())
        }
        Err(e) => {
            log.line(&format!("{e:#}"));
            log.finish("Failed");
            Err(e.context(format!("see: reims-ctl logs {name}")))
        }
    }
}

fn change_settings(name: &str) -> Result<()> {
    loop {
        header(&format!(
            "Settings of VM {}",
            ctl::parse_id(name).unwrap_or(0)
        ));
        summary(name);
        println!();
        item("M", "Memory", "", true);
        item("P", "Processors", "", true);
        item("U", "GPU", "which GPU runs the VM", true);
        item("R", "Resolution", "WIDTHxHEIGHT, or host", true);
        item("A", "Start with the computer", "on or off", true);
        item("E", "Eject the installer", "once macOS is installed", true);
        item("B", "Back", "", true);
        println!();
        match key("Choose an option: ") {
            'm' => {
                let v = ask("Memory, such as 8G or 16G", "8G").to_uppercase();
                ctl::run_ok(&["set", name, "--memory", &v])?;
            }
            'p' => {
                let v = ask("Processors (1 to 32)", "4");
                ctl::run_ok(&["set", name, "--cpus", &v])?;
            }
            'u' => {
                println!();
                let _ = ctl::run(&["gpus"]);
                let v = ask("\nGPU (auto, or a number from the list)", "auto");
                ctl::run_ok(&["set", name, "--gpu", &v])?;
            }
            'r' => {
                let v = ask("Resolution (WIDTHxHEIGHT or host)", "host");
                ctl::run_ok(&["set", name, "--resolution", &v])?;
            }
            'a' => toggle_autostart(name)?,
            'e' => {
                ctl::run_ok(&["eject", name])?;
                let mut meta = state::vm(name).unwrap_or_default();
                meta.first_start_done = true;
                state::save_vm(name, &meta)?;
            }
            'b' | 'q' | '\x1b' | '\n' => return Ok(()),
            _ => {}
        }
    }
}

fn toggle_autostart(name: &str) -> Result<()> {
    if state::autostart().as_deref() == Some(name) {
        state::set_autostart(None)?;
        good("✓ The VM no longer starts with the computer.\n");
    } else {
        let row = rows().into_iter().find(|r| r.entry.name == name);
        if !row.is_some_and(|r| r.installed()) {
            warn("Only a VM with macOS installed (and its installer ejected) can start with the computer.\n");
        } else {
            state::set_autostart(Some(name))?;
            good("✓ The VM starts with the computer (only one VM can).\n");
            dim("Set its screen to full screen; it opens right after the console.\n");
        }
    }
    pause();
    Ok(())
}

/// Início automático: a VM marcada sobe sozinha quando a sessão abre.
fn autostart_now() {
    let Some(name) = state::autostart() else {
        return;
    };
    if !ctl::installed() || blocker().is_some() {
        return;
    }
    let Some(row) = rows().into_iter().find(|r| r.entry.name == name) else {
        return;
    };
    if row.entry.running() || !row.installed() {
        return;
    }
    header("");
    println!("Starting VM {} automatically…", row.id());
    if let Ok(mut log) = state::ActionLog::start("autostart", Some(&name)) {
        match ctl::run_ok(&["start", &name]) {
            Ok(()) => log.finish("Done"),
            Err(e) => {
                log.line(&format!("{e:#}"));
                log.finish("Failed");
                bad(&format!("error: {e:#}\n"));
                pause();
            }
        }
    }
}

// ------------------------------------------------------------------ gerenciar

fn pick_vm(rows: &[Row], prompt: &str) -> Option<String> {
    if rows.is_empty() {
        warn("There are no VMs yet.\n");
        pause();
        return None;
    }
    let s = ask(prompt, "");
    let id = ctl::parse_id(&s)?;
    rows.iter()
        .find(|r| r.id() == id)
        .map(|r| r.entry.name.clone())
}

fn manage_vms() -> Result<()> {
    header("Manage VMs");
    let rows = rows();
    print_rows(&rows);
    let Some(name) = pick_vm(&rows, "VM number (Enter to go back)") else {
        return Ok(());
    };
    let row = rows.iter().find(|r| r.entry.name == name).unwrap();
    if !row.entry.running() {
        return start_vm(&name);
    }
    println!();
    item(
        "S",
        "Shut down",
        "macOS closes everything safely (recommended)",
        true,
    );
    item("R", "Restart", "forced, without saving", true);
    item("F", "Force power off", "cuts the VM's power", true);
    item(
        "T",
        "Terminate process",
        "when the VM does not respond",
        true,
    );
    item("C", "Cancel", "", true);
    println!();
    let k = key("Choose an option: ");
    let action = match k {
        's' => "stop",
        'r' => "restart",
        'f' => "poweroff",
        't' => "terminate",
        _ => return Ok(()),
    };
    let log = state::ActionLog::start(action, Some(&name))?;
    let res = match k {
        's' => ctl::run_ok(&["stop", &name]),
        'r' => {
            ctl::run_ok(&["stop", &name, "--force"]).and_then(|_| ctl::run_ok(&["start", &name]))
        }
        'f' => ctl::run_ok(&["stop", &name, "--force"]),
        _ => match row.entry.pid {
            Some(pid) => {
                let st = Command::new("kill")
                    .args(["-9", &pid.to_string()])
                    .status()?;
                if st.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("kill {pid} failed"))
                }
            }
            None => Err(anyhow::anyhow!("no process to terminate")),
        },
    };
    log.finish(if res.is_ok() { "Done" } else { "Failed" });
    res?;
    pause();
    Ok(())
}

// ------------------------------------------------------------------ logs

fn view_logs() -> Result<()> {
    loop {
        header("View logs");
        let logs = state::recent_logs(15);
        if logs.is_empty() {
            dim("  (no logs yet)\n");
            pause();
            return Ok(());
        }
        for (i, l) in logs.iter().enumerate() {
            print!("  ");
            accent(&format!("{:>2}", i + 1));
            print!("  {}  {:<14} {:<10} ", l.date, l.action, l.vm);
            match l.result.as_str() {
                "Done" => good(&l.result),
                "Failed" => bad(&l.result),
                _ => print!("{}", l.result),
            }
            println!();
        }
        dim(&format!("\nLogs are in {}\n", state::logs_dir().display()));
        let s = ask("Log number to open (Enter to go back)", "");
        let Some(l) = s
            .parse::<usize>()
            .ok()
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| logs.get(i))
        else {
            return Ok(());
        };
        if Command::new("less")
            .arg("-R")
            .arg(&l.path)
            .status()
            .is_err()
        {
            println!("{}", std::fs::read_to_string(&l.path).unwrap_or_default());
            pause();
        }
    }
}

// ------------------------------------------------------------------ manutenção

fn all_stopped() -> bool {
    let running: Vec<_> = rows().into_iter().filter(|r| r.entry.running()).collect();
    if running.is_empty() {
        return true;
    }
    warn("Shut every VM down first.\n");
    pause();
    false
}

fn maintenance() -> Result<()> {
    loop {
        header("Maintenance");
        item("1", "Delete a VM", "", true);
        item("2", "Delete all VMs", "", true);
        item("3", "Clean cache", "the downloaded macOS installers", true);
        item(
            "4",
            "Check this computer",
            "hardware, graphics and reims-ctl doctor",
            true,
        );
        item("5", "Prepare environment", "", true);
        item(
            "6",
            "Update Reims vGPU",
            "pinned or latest, with the VMs shut down",
            true,
        );
        item(
            "7",
            "Rebuild Reims vGPU",
            "the same version from scratch",
            true,
        );
        item("B", "Back", "", true);
        println!();
        let r = match key("Choose an option: ") {
            '1' => delete_vm(),
            '2' => delete_all(),
            '3' => clean_cache(),
            '4' => check_computer(),
            '5' => prepare(&[]).map(|_| ()),
            '6' => {
                if all_stopped() {
                    println!();
                    item(
                        "1",
                        "Pinned",
                        "the commit this version was made with (recommended)",
                        true,
                    );
                    item(
                        "2",
                        "Latest",
                        "the newest commit of Reims vGPU's master, not tested",
                        true,
                    );
                    let follow = match ask("Choose", "1").as_str() {
                        "2" => "--latest",
                        _ => "--pinned",
                    };
                    prepare(&["--update", follow]).map(|_| ())
                } else {
                    Ok(())
                }
            }
            '7' => {
                if all_stopped() {
                    prepare(&["--rebuild"]).map(|_| ())
                } else {
                    Ok(())
                }
            }
            'b' | 'q' | '\x1b' | '\n' => return Ok(()),
            _ => Ok(()),
        };
        if let Err(e) = r {
            bad(&format!("\nerror: {e:#}\n"));
            pause();
        }
    }
}

fn delete_vm() -> Result<()> {
    header("Delete a VM");
    let rows = rows();
    print_rows(&rows);
    let Some(name) = pick_vm(&rows, "VM number to delete (Enter to go back)") else {
        return Ok(());
    };
    if rows
        .iter()
        .any(|r| r.entry.name == name && r.entry.running())
    {
        warn("Shut the VM down first.\n");
        pause();
        return Ok(());
    }
    warn("Deleting a VM cannot be undone: macOS and every file in it are deleted.\n");
    if !confirm(&format!("Delete {name}?"), false) {
        return Ok(());
    }
    let mut log = state::ActionLog::start("delete", Some(&name))?;
    let r = ctl::run_ok(&["delete", &name, "--yes"]);
    if r.is_ok() {
        state::forget_vm(&name)?;
    }
    log.line(&format!("{r:?}"));
    log.finish(if r.is_ok() { "Done" } else { "Failed" });
    r?;
    pause();
    Ok(())
}

fn delete_all() -> Result<()> {
    header("Delete all VMs");
    let rows = rows();
    if rows.is_empty() {
        dim("  (no VMs created)\n");
        pause();
        return Ok(());
    }
    if !all_stopped() {
        return Ok(());
    }
    print_rows(&rows);
    warn("This deletes every VM above. It cannot be undone.\n");
    if !confirm("Delete all VMs?", false) {
        return Ok(());
    }
    let mut log = state::ActionLog::start("delete-all", None)?;
    for r in &rows {
        let res = ctl::run_ok(&["delete", &r.entry.name, "--yes"]);
        log.line(&format!("{}: {res:?}", r.entry.name));
        if res.is_ok() {
            state::forget_vm(&r.entry.name)?;
        }
    }
    log.finish("Done");
    pause();
    Ok(())
}

fn clean_cache() -> Result<()> {
    header("Clean cache");
    let dir = brand::home().join(".local/share/reims-ctl/installers");
    if !dir.exists() {
        dim("  The cache is empty.\n");
        pause();
        return Ok(());
    }
    let _ = Command::new("du").arg("-sh").arg(&dir).status();
    dim("VMs already created keep their own copy of the installer.\n");
    if !confirm("Delete the downloaded macOS installers?", false) {
        return Ok(());
    }
    let log = state::ActionLog::start("clean-cache", None)?;
    std::fs::remove_dir_all(&dir).with_context(|| format!("deleting {}", dir.display()))?;
    log.finish("Done");
    good("✓ Cache cleaned.\n");
    pause();
    Ok(())
}

fn check_computer() -> Result<()> {
    header("Check this computer");
    let mut log = state::ActionLog::start("doctor", None)?;
    let h = hw::detect();
    let text = hw::report(&h, &hw::plan(&h), &hw::validate(&h));
    println!("{text}");
    log.line(&text);
    if ctl::installed() {
        bold("reims-ctl doctor\n");
        let out = reimsctl_core::sys::capture(&ctl::bin().to_string_lossy(), &["doctor"]);
        println!("{out}");
        log.line(&out);
    }
    log.finish("Done");
    dim(&format!(
        "Saved to {} — share it when asking for help.\n",
        log_hint()
    ));
    pause();
    Ok(())
}

fn log_hint() -> String {
    state::logs_dir().join("latest.log").display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(state: &str, installer: bool, first: bool, source: &str) -> Row {
        Row {
            entry: ctl::VmEntry {
                name: "macos-10".into(),
                dir: PathBuf::new(),
                state: state.into(),
                pid: None,
            },
            meta: state::VmMeta {
                macos: "sonoma".into(),
                source: source.into(),
                first_start_done: first,
            },
            installer,
        }
    }

    #[test]
    fn states_follow_the_vm_life() {
        assert_eq!(
            row("stopped", true, false, "recovery").state(),
            "Ready to install"
        );
        assert_eq!(row("running", true, false, "recovery").state(), "Running");
        assert_eq!(
            row("stopped", false, false, "import").state(),
            "Ready for first use"
        );
        assert_eq!(row("stopped", false, true, "recovery").state(), "Installed");
        assert!(row("stopped", true, true, "recovery")
            .state()
            .starts_with("Installer attached"));
        assert!(row("broken", false, true, "recovery")
            .state()
            .starts_with("Faulty"));
    }

    #[test]
    fn only_installed_vms_can_autostart() {
        assert!(row("stopped", false, true, "recovery").installed());
        assert!(!row("stopped", true, true, "recovery").installed());
        assert!(!row("stopped", false, false, "import").installed());
    }

    #[test]
    fn macos_label() {
        assert_eq!(
            row("stopped", false, true, "x").macos(),
            "macOS Sonoma (14)"
        );
    }

    #[test]
    fn license_mentions_reims_vgpu_license() {
        assert!(LICENSE_TEXT.contains("GPL-2.0-or-later OR LGPL-3.0-or-later"));
    }
}
