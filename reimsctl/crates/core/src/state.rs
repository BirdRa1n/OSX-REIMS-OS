//! Estado do gerenciador, por usuário: licença, preparo do ambiente, metadados
//! das VMs, início automático e um log por ação.

use crate::brand;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Versão da licença; mudar o texto exige um novo aceite.
pub const LICENSE_VERSION: &str = "1.0";

/// Quantos logs de ação são mantidos.
pub const KEEP_LOGS: usize = 100;

fn dir() -> PathBuf {
    brand::user_state_dir()
}

fn read(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok()
}

fn write(p: &Path, s: &str) -> Result<()> {
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).with_context(|| format!("creating {}", d.display()))?;
    }
    fs::write(p, s).with_context(|| format!("writing {}", p.display()))
}

// ------------------------------------------------------------------ licença

pub fn license_accepted() -> bool {
    read(&dir().join("license")).is_some_and(|s| s.trim() == LICENSE_VERSION)
}

pub fn accept_license() -> Result<()> {
    write(&dir().join("license"), LICENSE_VERSION)
}

// ------------------------------------------------------------------ preparo

/// Gravado pelo `reims-prepare` ao concluir as 10 etapas.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prepared {
    /// Versão do appliance que preparou (outra versão pede um novo preparo).
    pub version: String,
    /// Commit do Reims vGPU compilado.
    pub commit: String,
    /// `pinned`, `latest` ou uma ref escolhida.
    pub follow: String,
    pub at: String,
}

pub fn prepared_path() -> PathBuf {
    dir().join("prepared.json")
}

pub fn prepared() -> Option<Prepared> {
    serde_json::from_str(&read(&prepared_path())?).ok()
}

// ------------------------------------------------------------------ VMs

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmMeta {
    /// Versão do macOS (`sonoma`…), ou `other` para um disco importado.
    pub macos: String,
    /// `recovery`, `offline`, `full` ou `import`.
    pub source: String,
    /// Já passou pela primeira inicialização (memória/CPUs respondidas).
    #[serde(default)]
    pub first_start_done: bool,
}

fn vms_path() -> PathBuf {
    dir().join("vms.json")
}

pub fn vms() -> BTreeMap<String, VmMeta> {
    read(&vms_path())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn vm(name: &str) -> Option<VmMeta> {
    vms().remove(name)
}

pub fn save_vm(name: &str, meta: &VmMeta) -> Result<()> {
    let mut all = vms();
    all.insert(name.to_string(), meta.clone());
    write(&vms_path(), &serde_json::to_string_pretty(&all)?)
}

pub fn forget_vm(name: &str) -> Result<()> {
    let mut all = vms();
    all.remove(name);
    write(&vms_path(), &serde_json::to_string_pretty(&all)?)?;
    if autostart().as_deref() == Some(name) {
        set_autostart(None)?;
    }
    Ok(())
}

// ------------------------------------------------------------------ início automático

fn autostart_path() -> PathBuf {
    dir().join("autostart")
}

/// A VM que inicia sozinha quando a sessão abre (só uma).
pub fn autostart() -> Option<String> {
    read(&autostart_path())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn set_autostart(name: Option<&str>) -> Result<()> {
    match name {
        Some(n) => write(&autostart_path(), n),
        None => {
            let _ = fs::remove_file(autostart_path());
            Ok(())
        }
    }
}

// ------------------------------------------------------------------ logs

pub fn logs_dir() -> PathBuf {
    dir().join("logs")
}

/// Um log por ação: cabeçalho (ação, VM, data), saída e resultado.
pub struct ActionLog {
    pub path: PathBuf,
    file: fs::File,
}

impl ActionLog {
    pub fn start(action: &str, vm: Option<&str>) -> Result<Self> {
        let d = logs_dir();
        fs::create_dir_all(&d).with_context(|| format!("creating {}", d.display()))?;
        let stamp = now_stamp();
        let name = format!(
            "{stamp}-{action}{}.log",
            vm.map(|v| format!("-{v}")).unwrap_or_default()
        );
        let path = d.join(name);
        // append: um script filho (o reims-prepare) escreve no mesmo arquivo.
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("creating {}", path.display()))?;
        writeln!(file, "action: {action}")?;
        writeln!(file, "vm: {}", vm.unwrap_or("-"))?;
        writeln!(file, "date: {stamp}")?;
        writeln!(file, "version: {} {}", brand::PRODUCT, brand::VERSION)?;
        writeln!(file, "----")?;
        let latest = d.join("latest.log");
        let _ = fs::remove_file(&latest);
        let _ = fs::copy(&path, &latest);
        prune(&d);
        Ok(ActionLog { path, file })
    }

    pub fn line(&mut self, s: &str) {
        let _ = writeln!(self.file, "{s}");
    }

    /// Fecha com um resultado: `Done`, `Nothing was done`, `Cancelled`, `Failed`.
    pub fn finish(mut self, result: &str) {
        let _ = writeln!(self.file, "----\nresult: {result}");
        let _ = fs::copy(&self.path, logs_dir().join("latest.log"));
    }
}

/// Um log já gravado, para a lista de `View logs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub path: PathBuf,
    pub action: String,
    pub vm: String,
    pub date: String,
    pub result: String,
}

/// Os logs mais recentes primeiro.
pub fn recent_logs(n: usize) -> Vec<LogEntry> {
    let Ok(rd) = fs::read_dir(logs_dir()) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "log")
                && p.file_name().is_some_and(|n| n != "latest.log")
        })
        .collect();
    files.sort();
    files.reverse();
    files
        .into_iter()
        .take(n)
        .map(|p| parse_log(&p, &read(&p).unwrap_or_default()))
        .collect()
}

pub fn parse_log(path: &Path, text: &str) -> LogEntry {
    let get = |k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(k))
            .map(|v| v.trim().to_string())
    };
    LogEntry {
        path: path.to_path_buf(),
        action: get("action:").unwrap_or_default(),
        vm: get("vm:").unwrap_or_default(),
        date: get("date:").unwrap_or_default(),
        // a última linha `result:` (o cabeçalho não tem uma).
        result: text
            .lines()
            .rev()
            .find_map(|l| l.strip_prefix("result:"))
            .map(|v| v.trim().to_string())
            .unwrap_or_else(|| "unfinished".into()),
    }
}

fn prune(d: &Path) {
    let Ok(rd) = fs::read_dir(d) else { return };
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n != "latest.log"))
        .collect();
    if files.len() <= KEEP_LOGS {
        return;
    }
    files.sort();
    for old in &files[..files.len() - KEEP_LOGS] {
        let _ = fs::remove_file(old);
    }
}

/// `AAAAMMDD-HHMMSS` em UTC, sem dependências de data.
pub fn now_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    stamp_from(secs)
}

fn stamp_from(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Dias desde 1970-01-01 → (ano, mês, dia) (algoritmo de Howard Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_known_dates() {
        assert_eq!(stamp_from(0), "19700101-000000");
        // 2026-10-14 12:34:56 UTC
        assert_eq!(stamp_from(1_791_981_296), "20261014-123456");
    }

    #[test]
    fn parse_log_result_or_unfinished() {
        let t =
            "action: create\nvm: macos-10\ndate: 20261014-000000\n----\nout\n----\nresult: Done\n";
        let e = parse_log(Path::new("x.log"), t);
        assert_eq!(e.action, "create");
        assert_eq!(e.vm, "macos-10");
        assert_eq!(e.result, "Done");
        let e = parse_log(Path::new("y.log"), "action: start\nvm: -\n");
        assert_eq!(e.result, "unfinished");
    }
}
