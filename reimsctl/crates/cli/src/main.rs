//! reimsctl — o gerenciador do appliance.
//!
//! Sem comando, abre o console. Tudo o que o console faz com as VMs passa pelo
//! `reims-ctl` do Reims vGPU; o resto (hardware, diagnóstico, versão) é daqui.

use anyhow::{bail, Context, Result};
use reimsctl_core::{brand, ctl, diag, hw, state, Verdict};
use std::process::Command;

const HELP: &str = "\
reimsctl — OSX-REIMS-OS manager

Usage: reimsctl [command] [options]

  console [--autostart]     the console (default); --autostart starts the VM marked to
                            start with the computer
  version                   versions of the system, kernel, reimsctl, reims-ctl and Reims vGPU
  hw detect [--json] [--save]
                            processor, virtualization, KVM, IOMMU, GPUs
  hw plan [--json|--packages|--nvidia-packages|--iommu-args]
                            what the first setup installs, and why (installs nothing)
  hw validate [--json] [--save]
                            the graphics chain: READY, SOFTWARE or FAIL (exit 0, 2, 1)
  hw report                 a readable summary of all of the above
  hw provision [--nvidia]   install the plan (root; the first setup runs it)
  diagnostics [--save] [--splash]
                            everything needed to understand a problem, safe to share
  doctor                    hw validate + reims-ctl doctor
  prepare [options]         prepare the environment (reims-prepare)
  license [--accept]        show (or accept) the license
  list                      the VMs
  start|stop|poweroff|delete N
                            act on VM N (10 to 99) through reims-ctl
  autostart [on N|off|status]
                            the VM that starts with the computer
  ctl ARGS...               run reims-ctl ARGS...
  help                      this help";

fn main() {
    match real_main() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    }
}

fn has(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn real_main() -> Result<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("console");
    let rest = &args[args.len().min(1)..];
    match cmd {
        "console" | "menu" => {
            reimsctl_tui::run(reimsctl_tui::Options {
                autostart: has(rest, "--autostart"),
            })?;
            Ok(0)
        }
        "version" | "--version" | "-V" => {
            print!("{}", diag::version_text());
            Ok(0)
        }
        "hw" => hw_cmd(rest),
        "diagnostics" => diagnostics(rest),
        "doctor" => doctor(),
        "prepare" | "setup" => {
            let st = Command::new("reims-prepare")
                .args(rest)
                .status()
                .context("running reims-prepare")?;
            Ok(st.code().unwrap_or(1))
        }
        "license" => {
            if has(rest, "--accept") {
                state::accept_license()?;
                println!("license {} accepted", state::LICENSE_VERSION);
            } else {
                print!("{}", reimsctl_tui::LICENSE_TEXT);
                println!(
                    "\naccepted: {}",
                    if state::license_accepted() {
                        "yes"
                    } else {
                        "no"
                    }
                );
            }
            Ok(0)
        }
        "list" => {
            ctl::run_ok(&["list"])?;
            Ok(0)
        }
        "start" | "stop" | "poweroff" | "delete" => {
            let n = rest
                .first()
                .and_then(|s| ctl::parse_id(s))
                .with_context(|| format!("usage: reimsctl {cmd} <VM number, 10 to 99>"))?;
            let name = ctl::vm_name(n);
            match cmd {
                "poweroff" => ctl::run_ok(&["stop", &name, "--force"])?,
                "delete" => {
                    ctl::run_ok(&["delete", &name])?;
                    state::forget_vm(&name)?;
                }
                c => ctl::run_ok(&[c, &name])?,
            }
            Ok(0)
        }
        "autostart" => autostart(rest),
        "ctl" => {
            let a: Vec<&str> = rest.iter().map(String::as_str).collect();
            Ok(ctl::run(&a)?.code().unwrap_or(1))
        }
        "help" | "-h" | "--help" => {
            println!("{HELP}");
            Ok(0)
        }
        other => {
            eprintln!("unknown command: {other}\n\n{HELP}");
            Ok(2)
        }
    }
}

fn save(path: &std::path::Path, text: &str) -> Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).with_context(|| format!("creating {}", d.display()))?;
    }
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

fn hw_cmd(args: &[String]) -> Result<i32> {
    let sub = args.first().map(String::as_str).unwrap_or("report");
    let json = has(args, "--json");
    let h = hw::detect();
    match sub {
        "detect" => {
            let j = serde_json::to_string_pretty(&h)?;
            if has(args, "--save") {
                save(&hw::saved_path(), &j)?;
            }
            if json {
                println!("{j}");
            } else {
                print!("{}", detect_text(&h));
            }
            Ok(0)
        }
        "plan" => {
            let p = hw::plan(&h);
            if json {
                println!("{}", serde_json::to_string_pretty(&p)?);
            } else if has(args, "--packages") {
                println!("{}", p.packages.join("\n"));
            } else if has(args, "--nvidia-packages") {
                println!("{}", p.nvidia_packages.join("\n"));
            } else if has(args, "--iommu-args") {
                println!("{}", p.iommu_args.join(" "));
            } else {
                println!("Drivers and firmware, from Debian's repository:");
                for r in &p.reasons {
                    println!("  {r}");
                }
                println!("\nPackages: {}", p.packages.join(" "));
                if !p.nvidia_packages.is_empty() {
                    println!(
                        "NVIDIA (only with your permission): {}",
                        p.nvidia_packages.join(" ")
                    );
                }
                println!("Kernel modules: {}", p.modules.join(" "));
                if !p.iommu_args.is_empty() {
                    println!(
                        "IOMMU (optional, not needed by Reims vGPU): {}",
                        p.iommu_args.join(" ")
                    );
                }
            }
            Ok(0)
        }
        "validate" => {
            let v = hw::validate(&h);
            if has(args, "--save") {
                save(
                    &hw::verdict_path(),
                    &format!("{}\n{}\n", v.verdict.as_str(), v.message),
                )?;
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                for c in &v.gpus {
                    println!(
                        "{} {}{} — {}",
                        if c.ok { "✓" } else { "✗" },
                        c.name,
                        if c.display { " (monitor)" } else { "" },
                        c.detail
                    );
                }
                println!(
                    "{} KVM{}",
                    if v.kvm { "✓" } else { "✗" },
                    if v.kvm {
                        String::new()
                    } else {
                        " — /dev/kvm is missing: enable VT-x/AMD-V in the firmware".into()
                    }
                );
                println!("{}: {}", v.verdict.as_str(), v.message);
            }
            Ok(match v.verdict {
                Verdict::Ready => 0,
                Verdict::Software => 2,
                Verdict::Fail => 1,
            })
        }
        "report" => {
            print!("{}", hw::report(&h, &hw::plan(&h), &hw::validate(&h)));
            Ok(0)
        }
        "provision" | "drivers" => {
            let p = hw::plan(&h);
            let mut pkgs = p.packages.clone();
            if has(args, "--nvidia") {
                pkgs.extend(p.nvidia_packages.clone());
            }
            println!("Installing: {}", pkgs.join(" "));
            let st = Command::new("apt-get")
                .args(["install", "-y", "--no-install-recommends"])
                .args(&pkgs)
                .status()
                .context("running apt-get (as root)")?;
            Ok(st.code().unwrap_or(1))
        }
        other => bail!("unknown: hw {other} (detect, plan, validate, report, provision)"),
    }
}

fn detect_text(h: &hw::Hardware) -> String {
    let yes = |b: bool| if b { "yes" } else { "no" };
    let mut s = format!(
        "Processor      {}\nVirtualization {}\nKVM            {}\nIOMMU          firmware {}, enabled {}\n",
        h.cpu.model,
        h.cpu.virt.as_deref().unwrap_or("none"),
        yes(h.kvm),
        yes(h.iommu.firmware),
        yes(h.iommu.enabled)
    );
    for (i, g) in h.gpus.iter().enumerate() {
        s.push_str(&format!(
            "GPU{i}           {} [{}] driver {}{}\n",
            g.name,
            g.id(),
            g.driver.as_deref().unwrap_or("none"),
            if g.display { " · monitor" } else { "" }
        ));
    }
    s
}

fn diagnostics(args: &[String]) -> Result<i32> {
    if has(args, "--splash") {
        print!("{}", diag::splash_text());
        return Ok(0);
    }
    let r = diag::report();
    print!("{r}");
    if has(args, "--save") {
        let p = brand::home().join(format!("reims-diagnostics-{}.txt", state::now_stamp()));
        save(&p, &r)?;
        eprintln!("\nsaved: {}", p.display());
    }
    Ok(0)
}

fn doctor() -> Result<i32> {
    let h = hw::detect();
    let v = hw::validate(&h);
    println!("{}: {}", v.verdict.as_str(), v.message);
    let mut code = i32::from(v.verdict != Verdict::Ready);
    if ctl::installed() {
        code |= ctl::run(&["doctor"])?.code().unwrap_or(1);
    } else {
        println!("reims-ctl is not installed: prepare the environment (reimsctl prepare)");
        code = 1;
    }
    Ok(code)
}

fn autostart(args: &[String]) -> Result<i32> {
    match args.first().map(String::as_str).unwrap_or("status") {
        "on" => {
            let n = args
                .get(1)
                .and_then(|s| ctl::parse_id(s))
                .context("usage: reimsctl autostart on <VM number>")?;
            state::set_autostart(Some(&ctl::vm_name(n)))?;
            println!("VM {n} starts with the computer");
        }
        "off" => {
            state::set_autostart(None)?;
            println!("no VM starts with the computer");
        }
        _ => match state::autostart() {
            Some(n) => println!("{n} starts with the computer"),
            None => println!("no VM starts with the computer"),
        },
    }
    Ok(0)
}
