//! Caminhos de runtime do appliance (com overrides por variável de ambiente,
//! úteis em desenvolvimento).

use crate::release::MacosRelease;
use std::path::{Path, PathBuf};

/// Bases de OpenCore instaladas na imagem: `<dir>/<rail>/OpenCore.qcow2`.
pub const DEFAULT_OPENCORE_BASE_DIR: &str = "/usr/local/share/reims/opencore";

/// Helper que extrai/injeta o `config.plist` de dentro da OpenCore.qcow2.
pub const DEFAULT_OC_IMAGE_SCRIPT: &str = "/usr/local/lib/reims/oc-image.sh";

/// Diretório base das árvores de OpenCore (`REIMS_OPENCORE_BASE_DIR`).
pub fn opencore_base_dir() -> PathBuf {
    std::env::var_os("REIMS_OPENCORE_BASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OPENCORE_BASE_DIR))
}

/// Imagem base de OpenCore para uma versão: `<base_dir>/<rail>/OpenCore.qcow2`.
pub fn opencore_base_image(base_dir: &Path, release: MacosRelease) -> PathBuf {
    base_dir.join(release.rail()).join("OpenCore.qcow2")
}

/// Caminho do `oc-image.sh` (`REIMS_OC_IMAGE_SCRIPT`).
pub fn oc_image_script() -> PathBuf {
    std::env::var_os("REIMS_OC_IMAGE_SCRIPT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OC_IMAGE_SCRIPT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencore_image_path_uses_rail() {
        let base = Path::new("/srv/oc");
        let p = opencore_base_image(base, MacosRelease::Sonoma);
        assert!(p.ends_with("macos-14/OpenCore.qcow2"), "{}", p.display());
    }
}
