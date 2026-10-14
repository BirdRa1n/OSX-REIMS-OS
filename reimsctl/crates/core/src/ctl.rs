//! Ponte para o `reims-ctl` do upstream (Reims vGPU).
//!
//! O `reims-ctl` cria as VMs (baixa o macOS da Apple, gera a imagem do OpenCore
//! com identidade própria e o disco), sobe o QEMU do Reims vGPU e o desliga.
//! Este módulo só o chama: as VMs são dele, com nomes `macos-<número>`.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

/// Prefixo dos nomes das VMs (`macos-10` … `macos-99`).
pub const VM_PREFIX: &str = "macos-";
pub const MIN_ID: u32 = 10;
pub const MAX_ID: u32 = 99;

/// O binário do `reims-ctl`: `REIMS_CTL`, `~/.cargo/bin/reims-ctl` ou o do PATH.
pub fn bin() -> PathBuf {
    if let Some(p) = std::env::var_os("REIMS_CTL") {
        return PathBuf::from(p);
    }
    let cargo = crate::brand::home().join(".cargo/bin/reims-ctl");
    if cargo.is_file() {
        return cargo;
    }
    PathBuf::from("reims-ctl")
}

pub fn installed() -> bool {
    Command::new(bin())
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// `reims-ctl --version`, se instalado.
pub fn version() -> Option<String> {
    let out = Command::new(bin()).arg("--version").output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Roda com o terminal do usuário (perguntas, progresso).
pub fn run(args: &[&str]) -> Result<ExitStatus> {
    Command::new(bin())
        .args(args)
        .status()
        .with_context(|| format!("running {} {}", bin().display(), args.join(" ")))
}

/// Roda e exige sucesso.
pub fn run_ok(args: &[&str]) -> Result<()> {
    let st = run(args)?;
    if !st.success() {
        bail!("reims-ctl {} failed ({st})", args.join(" "));
    }
    Ok(())
}

/// Saída de texto de um comando.
pub fn output(args: &[&str]) -> Result<String> {
    let out = Command::new(bin())
        .args(args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("running {} {}", bin().display(), args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "reims-ctl {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VmEntry {
    pub name: String,
    #[serde(default)]
    pub dir: PathBuf,
    /// `running`, `stopped` ou `broken`.
    pub state: String,
    #[serde(default)]
    pub pid: Option<u32>,
}

impl VmEntry {
    pub fn id(&self) -> Option<u32> {
        parse_id(&self.name)
    }
    pub fn running(&self) -> bool {
        self.state == "running"
    }
}

/// `reims-ctl list --json`.
pub fn list() -> Result<Vec<VmEntry>> {
    parse_list(&output(&["list", "--json"])?)
}

pub fn parse_list(json: &str) -> Result<Vec<VmEntry>> {
    let mut v: Vec<VmEntry> = serde_json::from_str(json).context("parsing reims-ctl list")?;
    v.sort_by_key(|e| (e.id().unwrap_or(u32::MAX), e.name.clone()));
    Ok(v)
}

/// `reims-ctl show NAME`, como pares (chave, valor).
pub fn show(name: &str) -> Result<Vec<(String, String)>> {
    Ok(parse_show(&output(&["show", name])?))
}

/// As linhas `  chave        valor` do `show` (a primeira linha é o nome).
pub fn parse_show(text: &str) -> Vec<(String, String)> {
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let l = l.trim();
            // a chave pode ter espaço ("cpu model"); o valor começa após 2+ espaços.
            let i = l.find("  ")?;
            Some((l[..i].trim().to_string(), l[i..].trim().to_string()))
        })
        .collect()
}

pub fn vm_name(id: u32) -> String {
    format!("{VM_PREFIX}{id}")
}

/// `macos-12` (ou `12`) → 12.
pub fn parse_id(s: &str) -> Option<u32> {
    let s = s.trim();
    let n = s.strip_prefix(VM_PREFIX).unwrap_or(s);
    n.parse().ok().filter(|i| (MIN_ID..=MAX_ID).contains(i))
}

/// O primeiro número livre de 10 a 99.
pub fn first_free(used: &[u32]) -> Option<u32> {
    (MIN_ID..=MAX_ID).find(|i| !used.contains(i))
}

/// Porta SSH do guest: `22<número>` (VM 10 → 2210).
pub fn ssh_port(id: u32) -> u16 {
    (2200 + id) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_json_sorted_by_id() {
        let j = r#"[{"name":"macos-12","dir":"/x","state":"stopped","pid":null},
                    {"name":"macos-10","dir":"/y","state":"running","pid":42}]"#;
        let l = parse_list(j).unwrap();
        assert_eq!(l[0].name, "macos-10");
        assert!(l[0].running());
        assert_eq!(l[1].id(), Some(12));
    }

    #[test]
    fn show_pairs_keep_spaced_keys() {
        let t = "macos-10\n  state         stopped\n  memory        8G\n  cpu model     Skylake\n  installer     none\n";
        let p = parse_show(t);
        assert_eq!(p[0], ("state".into(), "stopped".into()));
        assert_eq!(p[2], ("cpu model".into(), "Skylake".into()));
        assert_eq!(p[3].1, "none");
    }

    #[test]
    fn ids_and_ports() {
        assert_eq!(parse_id("macos-42"), Some(42));
        assert_eq!(parse_id("7"), None);
        assert_eq!(parse_id("other"), None);
        assert_eq!(first_free(&[10, 11, 13]), Some(12));
        assert_eq!(ssh_port(10), 2210);
        assert_eq!(vm_name(15), "macos-15");
    }
}
