//! Utilitários para rodar comandos do sistema sem quebrar quando eles faltam.

use std::process::{Command, Stdio};

/// Saída (stdout) de um comando, ou `None` se ele não existe ou falhou.
pub fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Saída (stdout + stderr) de um comando, mesmo com status de erro; marca a
/// ausência do comando no texto. Usado pelo diagnóstico.
pub fn capture(cmd: &str, args: &[&str]) -> String {
    match Command::new(cmd).args(args).stdin(Stdio::null()).output() {
        Ok(out) => {
            let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
            let err = String::from_utf8_lossy(&out.stderr);
            if !err.trim().is_empty() {
                s.push_str(&err);
            }
            if !out.status.success() {
                s.push_str(&format!("(exit status: {})\n", out.status));
            }
            s
        }
        Err(_) => format!("({cmd}: not available)\n"),
    }
}

/// `true` se o comando existe no PATH.
pub fn has(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}
