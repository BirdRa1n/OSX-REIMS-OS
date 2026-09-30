//! reimsctl-efi — geração de identidade única por VM e injeção do OpenCore.
//!
//! Cada VM recebe serial/MLB/UUID/ROM/MAC próprios para não colidir entre
//! instâncias (mesmo princípio das "400+ customizações" do OSX-REIMS). Estes
//! geradores produzem valores **com o formato certo, porém aleatórios** — bons
//! para isolamento entre VMs, não para "passar" como um Mac específico.

use anyhow::{Context, Result};
use plist::{Dictionary, Value};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Identidade de máquina injetada no `config.plist` do OpenCore (PlatformInfo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub serial: String,
    pub mlb: String,
    /// SystemUUID (SmUUID) no formato canônico 8-4-4-4-12.
    pub uuid: String,
    /// ROM = 6 bytes em hex (normalmente derivado do MAC).
    pub rom: String,
    /// MAC address `xx:xx:xx:xx:xx:xx` (locally administered).
    pub mac: String,
}

impl Identity {
    /// Gera uma identidade aleatória e internamente consistente (ROM ← MAC).
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();

        let mac_bytes = random_mac_bytes(&mut rng);
        let mac = mac_bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":");
        let rom = mac_bytes.iter().map(|b| format!("{b:02x}")).collect();

        Identity {
            serial: random_serial(&mut rng, 12),
            mlb: random_serial(&mut rng, 17),
            uuid: random_uuid(&mut rng),
            rom,
            mac,
        }
    }

    /// Decodifica o ROM (12 chars hex) para os 6 bytes crus usados no plist.
    pub fn rom_bytes(&self) -> Result<[u8; 6]> {
        anyhow::ensure!(
            self.rom.len() == 12,
            "ROM deve ter 12 caracteres hex, tem {}",
            self.rom.len()
        );
        let mut out = [0u8; 6];
        for (i, chunk) in self.rom.as_bytes().chunks(2).enumerate() {
            let s = std::str::from_utf8(chunk).context("ROM não é UTF-8")?;
            out[i] = u8::from_str_radix(s, 16).with_context(|| format!("ROM hex inválido: {s}"))?;
        }
        Ok(out)
    }
}

/// MAC "locally administered" e unicast (bit 1 do 1º octeto = 1, bit 0 = 0).
fn random_mac_bytes<R: Rng>(rng: &mut R) -> [u8; 6] {
    let mut b = [0u8; 6];
    rng.fill(&mut b);
    b[0] = (b[0] & 0b1111_1100) | 0b0000_0010;
    b
}

/// Serial pseudo-aleatório do alfabeto usado pela Apple (sem I/O/etc.).
fn random_serial<R: Rng>(rng: &mut R, len: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ0123456789";
    (0..len)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

/// UUIDv4-like no formato canônico.
fn random_uuid<R: Rng>(rng: &mut R) -> String {
    let mut b = [0u8; 16];
    rng.fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40; // versão 4
    b[8] = (b[8] & 0x3f) | 0x80; // variante
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

/// Garante que `parent[key]` seja um dicionário e o retorna (cria se faltar).
fn ensure_dict<'a>(parent: &'a mut Dictionary, key: &str) -> Result<&'a mut Dictionary> {
    if !parent.contains_key(key) {
        parent.insert(key.to_string(), Value::Dictionary(Dictionary::new()));
    }
    parent
        .get_mut(key)
        .and_then(Value::as_dictionary_mut)
        .with_context(|| format!("config.plist: '{key}' não é um dicionário"))
}

/// Escreve a identidade em `PlatformInfo → Generic` de um `config.plist` do
/// OpenCore (XML plist), preservando todo o resto do arquivo.
///
/// Define `SystemSerialNumber`, `MLB`, `SystemUUID` e `ROM` (bytes crus). Não
/// mexe em `SystemProductName` (o modelo SMBIOS vem da árvore base do rail).
///
/// Extrair o `config.plist` de dentro do `OpenCore.qcow2` e reempacotar é
/// tarefa do host (mtools/libguestfs) — ver `opencore/README.md`; esta função
/// opera sobre o arquivo já extraído.
pub fn set_platform_identity(config_path: &Path, id: &Identity) -> Result<()> {
    let mut root = Value::from_file(config_path)
        .with_context(|| format!("lendo plist {}", config_path.display()))?;
    let dict = root
        .as_dictionary_mut()
        .context("config.plist: raiz não é um dicionário")?;

    let platform = ensure_dict(dict, "PlatformInfo")?;
    let generic = ensure_dict(platform, "Generic")?;

    generic.insert(
        "SystemSerialNumber".into(),
        Value::String(id.serial.clone()),
    );
    generic.insert("MLB".into(), Value::String(id.mlb.clone()));
    generic.insert("SystemUUID".into(), Value::String(id.uuid.clone()));
    generic.insert("ROM".into(), Value::Data(id.rom_bytes()?.to_vec()));

    root.to_file_xml(config_path)
        .with_context(|| format!("gravando plist {}", config_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_has_correct_shapes() {
        let id = Identity::generate();
        assert_eq!(id.serial.len(), 12);
        assert_eq!(id.mlb.len(), 17);
        assert_eq!(id.uuid.len(), 36); // 8-4-4-4-12 + 4 hífens
        assert_eq!(id.uuid.matches('-').count(), 4);
        assert_eq!(id.rom.len(), 12); // 6 bytes em hex
        assert_eq!(id.mac.matches(':').count(), 5);
    }

    #[test]
    fn mac_is_locally_administered_unicast() {
        let id = Identity::generate();
        let first = u8::from_str_radix(&id.mac[0..2], 16).unwrap();
        assert_eq!(first & 0b11, 0b10); // bit local = 1, bit multicast = 0
    }

    #[test]
    fn rom_matches_mac_bytes() {
        let id = Identity::generate();
        let mac_hex: String = id.mac.split(':').collect();
        assert_eq!(id.rom, mac_hex);
    }

    #[test]
    fn rom_bytes_decodes_hex() {
        let id = Identity {
            serial: "X".into(),
            mlb: "Y".into(),
            uuid: "Z".into(),
            rom: "0a1b2c3d4e5f".into(),
            mac: "0a:1b:2c:3d:4e:5f".into(),
        };
        assert_eq!(
            id.rom_bytes().unwrap(),
            [0x0a, 0x1b, 0x2c, 0x3d, 0x4e, 0x5f]
        );
    }

    fn temp_dir() -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "reimsctl-efi-test-{}-{}",
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn injects_identity_and_preserves_other_keys() {
        // fixture: config.plist com PlatformInfo/Generic e uma chave a preservar
        let mut generic = Dictionary::new();
        generic.insert("SystemSerialNumber".into(), Value::String("OLD".into()));
        let mut platform = Dictionary::new();
        platform.insert("Automatic".into(), Value::Boolean(true));
        platform.insert("Generic".into(), Value::Dictionary(generic));
        let mut root = Dictionary::new();
        root.insert("PlatformInfo".into(), Value::Dictionary(platform));
        root.insert("Kernel".into(), Value::Dictionary(Dictionary::new()));

        let dir = temp_dir();
        let path = dir.join("config.plist");
        Value::Dictionary(root).to_file_xml(&path).unwrap();

        let id = Identity::generate();
        set_platform_identity(&path, &id).unwrap();

        let out = Value::from_file(&path).unwrap();
        let d = out.as_dictionary().unwrap();
        assert!(d.contains_key("Kernel"), "outras chaves preservadas");

        let g = d
            .get("PlatformInfo")
            .unwrap()
            .as_dictionary()
            .unwrap()
            .get("Generic")
            .unwrap()
            .as_dictionary()
            .unwrap();
        assert_eq!(
            g.get("SystemSerialNumber").unwrap().as_string().unwrap(),
            id.serial
        );
        assert_eq!(g.get("MLB").unwrap().as_string().unwrap(), id.mlb);
        assert_eq!(g.get("SystemUUID").unwrap().as_string().unwrap(), id.uuid);
        assert_eq!(
            g.get("ROM").unwrap().as_data().unwrap(),
            id.rom_bytes().unwrap().as_slice()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn creates_platform_info_when_missing() {
        let dir = temp_dir();
        let path = dir.join("config.plist");
        // plist mínimo, sem PlatformInfo
        Value::Dictionary(Dictionary::new())
            .to_file_xml(&path)
            .unwrap();

        let id = Identity::generate();
        set_platform_identity(&path, &id).unwrap();

        let out = Value::from_file(&path).unwrap();
        let g = out
            .as_dictionary()
            .unwrap()
            .get("PlatformInfo")
            .unwrap()
            .as_dictionary()
            .unwrap()
            .get("Generic")
            .unwrap()
            .as_dictionary()
            .unwrap();
        assert_eq!(g.get("MLB").unwrap().as_string().unwrap(), id.mlb);

        std::fs::remove_dir_all(&dir).ok();
    }
}
