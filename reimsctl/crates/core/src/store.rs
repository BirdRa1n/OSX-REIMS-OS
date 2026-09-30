//! Persistência e ciclo de vida das VMs no disco.
//!
//! Cada VM vive num diretório próprio sob a raiz do store (por padrão
//! `/var/lib/reims/vms/<nome>`), com o `config.json` como fonte da verdade.

use crate::release::MacosRelease;
use crate::vm::VmConfig;
use anyhow::{ensure, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Repositório de VMs enraizado num diretório.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Raiz padrão no appliance.
    pub fn default_root() -> PathBuf {
        PathBuf::from("/var/lib/reims/vms")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn vm_dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// Cria uma nova VM (diretório + `config.json`). Falha se já existir.
    pub fn create(&self, name: &str, macos: MacosRelease) -> Result<VmConfig> {
        let dir = self.vm_dir(name);
        ensure!(!dir.exists(), "VM já existe: {name}");
        fs::create_dir_all(&dir).with_context(|| format!("criando {}", dir.display()))?;
        let cfg = VmConfig::new(name, macos, dir);
        self.save(&cfg)?;
        Ok(cfg)
    }

    /// Grava (ou regrava) o `config.json` da VM.
    pub fn save(&self, cfg: &VmConfig) -> Result<()> {
        let json = serde_json::to_string_pretty(cfg)?;
        let path = cfg.config_path();
        fs::write(&path, json).with_context(|| format!("salvando {}", path.display()))?;
        Ok(())
    }

    /// Carrega a config de uma VM pelo nome.
    pub fn load(&self, name: &str) -> Result<VmConfig> {
        let path = self.vm_dir(name).join("config.json");
        let data =
            fs::read_to_string(&path).with_context(|| format!("lendo {}", path.display()))?;
        let cfg =
            serde_json::from_str(&data).with_context(|| format!("parseando {}", path.display()))?;
        Ok(cfg)
    }

    /// Lista os nomes de VMs (diretórios com `config.json`), ordenados.
    pub fn list(&self) -> Result<Vec<String>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in
            fs::read_dir(&self.root).with_context(|| format!("listando {}", self.root.display()))?
        {
            let entry = entry?;
            if entry.path().join("config.json").is_file() {
                if let Some(n) = entry.file_name().to_str() {
                    names.push(n.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// Remove uma VM e todo o seu diretório. Falha se não existir.
    pub fn delete(&self, name: &str) -> Result<()> {
        let dir = self.vm_dir(name);
        ensure!(dir.exists(), "VM não existe: {name}");
        fs::remove_dir_all(&dir).with_context(|| format!("removendo {}", dir.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p =
            std::env::temp_dir().join(format!("reimsctl-test-{}-{}", std::process::id(), nanos));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn create_list_load_delete_roundtrip() {
        let root = temp_root();
        let store = Store::new(&root);

        assert!(store.list().unwrap().is_empty());

        let cfg = store.create("alpha", MacosRelease::Sonoma).unwrap();
        assert_eq!(cfg.name, "alpha");
        assert_eq!(cfg.macos, MacosRelease::Sonoma);
        assert!(cfg.config_path().is_file());

        // criar duplicada falha
        assert!(store.create("alpha", MacosRelease::Sonoma).is_err());

        store.create("beta", MacosRelease::Ventura).unwrap();
        assert_eq!(store.list().unwrap(), vec!["alpha", "beta"]);

        let loaded = store.load("alpha").unwrap();
        assert_eq!(loaded.name, "alpha");
        assert_eq!(loaded.disk_gb, cfg.disk_gb);

        store.delete("alpha").unwrap();
        assert_eq!(store.list().unwrap(), vec!["beta"]);
        assert!(store.load("alpha").is_err());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn list_missing_root_is_empty() {
        let store = Store::new(temp_root().join("does-not-exist"));
        assert!(store.list().unwrap().is_empty());
    }
}
