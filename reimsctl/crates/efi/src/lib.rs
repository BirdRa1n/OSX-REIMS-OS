//! reimsctl-efi — geração de identidade única por VM e injeção do OpenCore.
//!
//! Cada VM recebe serial/MLB/UUID/ROM/MAC próprios para não colidir entre
//! instâncias (mesmo princípio das "400+ customizações" do OSX-REIMS). Estes
//! geradores produzem valores **com o formato certo, porém aleatórios** — bons
//! para isolamento entre VMs, não para "passar" como um Mac específico.

use rand::Rng;
use serde::{Deserialize, Serialize};

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

/// Injeta a identidade num template de OpenCore.
///
/// TODO(fase-1): abrir a árvore base de `opencore/<rail>/`, editar o
/// `config.plist` (PlatformInfo/Generic) e empacotar como `OpenCore.qcow2`.
pub fn inject_opencore(_rail: &str, _id: &Identity) -> anyhow::Result<()> {
    anyhow::bail!("inject_opencore: não implementado (fase-1)")
}
