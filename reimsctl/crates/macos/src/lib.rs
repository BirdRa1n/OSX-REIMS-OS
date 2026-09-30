//! reimsctl-macos — download da mídia de instalação do macOS.
//!
//! Fala o protocolo do `osrecovery.apple.com` (mesma abordagem do `fetch-macOS`
//! do ecossistema OSX-KVM): abre uma sessão, pede a **imagem de Recovery** de um
//! board-id e baixa `BaseSystem.dmg` + `.chunklist` **direto da Apple**. Nenhuma
//! imagem é redistribuída por este projeto — o download roda na máquina do usuário.

use anyhow::{ensure, Context, Result};
use rand::Rng;
use reimsctl_core::release::MacosRelease;
use std::fs::File;
use std::path::{Path, PathBuf};

const OSRECOVERY_ROOT: &str = "https://osrecovery.apple.com/";
const RECOVERY_ENDPOINT: &str = "https://osrecovery.apple.com/InstallationPayload/RecoveryImage";
const USER_AGENT: &str = "InstallAssistant/1.0";

/// Board-id padrão (iMacPro1,1) — devolve um macOS recente via osrecovery.
pub const DEFAULT_BOARD_ID: &str = "Mac-7BA5B2D9E42DDD94";

/// Board-id usado para pedir o Recovery de uma versão.
///
/// TODO(fase-0): mapear cada versão para um board-id que entregue exatamente
/// aquela release (o osrecovery devolve o Recovery *default/latest* do board).
/// Por ora todas usam o board padrão (macOS recente).
pub fn board_id_for(_release: MacosRelease) -> &'static str {
    DEFAULT_BOARD_ID
}

/// Links assinados retornados pelo osrecovery para baixar a imagem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryLinks {
    pub asset_url: String,       // AU
    pub asset_token: String,     // AT
    pub chunklist_url: String,   // CU
    pub chunklist_token: String, // CT
}

/// Resultado de um download bem-sucedido.
#[derive(Debug, Clone)]
pub struct RecoveryImage {
    pub release: MacosRelease,
    pub path: PathBuf,
}

/// Gera uma string hex aleatória de `len` caracteres (nonce do protocolo).
fn rand_hex(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| char::from_digit(rng.gen_range(0..16), 16).unwrap())
        .collect()
}

/// Monta o corpo (text/plain) do POST de RecoveryImage.
fn build_request_body(
    board_id: &str,
    os_type: &str,
    cid: &str,
    k: &str,
    fg: &str,
    sn: &str,
) -> String {
    format!("cid={cid}\nsn={sn}\nbid={board_id}\nk={k}\nos={os_type}\nfg={fg}\n")
}

/// Faz o parse da resposta do osrecovery (`AU`/`AT`/`CU`/`CT`), aceitando `=`
/// ou `:` como separador.
fn parse_response(text: &str) -> Result<RecoveryLinks> {
    let (mut au, mut at, mut cu, mut ct) = (None, None, None, None);
    for line in text.lines() {
        let line = line.trim();
        let Some(idx) = line.find(['=', ':']) else {
            continue;
        };
        let key = line[..idx].trim();
        let val = line[idx + 1..].trim().to_string();
        match key {
            "AU" => au = Some(val),
            "AT" => at = Some(val),
            "CU" => cu = Some(val),
            "CT" => ct = Some(val),
            _ => {}
        }
    }
    Ok(RecoveryLinks {
        asset_url: au.context("resposta sem AU (asset url)")?,
        asset_token: at.context("resposta sem AT (asset token)")?,
        chunklist_url: cu.context("resposta sem CU (chunklist url)")?,
        chunklist_token: ct.context("resposta sem CT (chunklist token)")?,
    })
}

/// Abre uma sessão e retorna o cookie `session=...` a ser reenviado.
fn get_session(agent: &ureq::Agent) -> Result<String> {
    let resp = agent
        .get(OSRECOVERY_ROOT)
        .set("User-Agent", USER_AGENT)
        .call()
        .context("abrindo sessão no osrecovery")?;
    let cookie = resp
        .all("Set-Cookie")
        .into_iter()
        .find(|c| c.contains("session="))
        .context("osrecovery não devolveu cookie de sessão")?;
    let session = cookie.split(';').next().unwrap_or(cookie).to_string();
    Ok(session)
}

/// Pede os links da imagem de Recovery para um board-id.
fn request_image(agent: &ureq::Agent, session: &str, board_id: &str) -> Result<RecoveryLinks> {
    let body = build_request_body(
        board_id,
        "latest",
        &rand_hex(16),
        &rand_hex(64),
        &rand_hex(64),
        "",
    );
    let resp = agent
        .post(RECOVERY_ENDPOINT)
        .set("User-Agent", USER_AGENT)
        .set("Cookie", session)
        .set("Content-Type", "text/plain")
        .send_string(&body)
        .context("pedindo RecoveryImage")?;
    let text = resp.into_string().context("lendo resposta do osrecovery")?;
    parse_response(&text)
}

/// Baixa `url` (com o `AssetToken`) para `dest`, em streaming.
fn download(agent: &ureq::Agent, url: &str, token: &str, dest: &Path) -> Result<()> {
    let resp = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .set("Cookie", &format!("AssetToken={token}"))
        .call()
        .with_context(|| format!("baixando {url}"))?;
    let mut reader = resp.into_reader();
    let mut file = File::create(dest).with_context(|| format!("criando {}", dest.display()))?;
    std::io::copy(&mut reader, &mut file)
        .with_context(|| format!("gravando {}", dest.display()))?;
    Ok(())
}

/// Baixa a imagem de Recovery da versão pedida para `dest_dir`
/// (`BaseSystem.dmg` + `BaseSystem.chunklist`).
pub fn fetch_recovery(release: MacosRelease, dest_dir: &Path) -> Result<RecoveryImage> {
    ensure!(
        dest_dir.is_dir(),
        "destino não é um diretório: {}",
        dest_dir.display()
    );
    let agent = ureq::agent();
    let session = get_session(&agent)?;
    let links = request_image(&agent, &session, board_id_for(release))?;

    let base = dest_dir.join("BaseSystem.dmg");
    let chunklist = dest_dir.join("BaseSystem.chunklist");
    download(&agent, &links.asset_url, &links.asset_token, &base)?;
    download(
        &agent,
        &links.chunklist_url,
        &links.chunklist_token,
        &chunklist,
    )?;

    Ok(RecoveryImage {
        release,
        path: base,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rand_hex_shape() {
        let s = rand_hex(64);
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn request_body_has_all_fields() {
        let b = build_request_body("Mac-XYZ", "latest", "CID", "KKK", "FGG", "");
        assert!(b.contains("bid=Mac-XYZ"));
        assert!(b.contains("os=latest"));
        assert!(b.contains("cid=CID"));
        assert!(b.contains("k=KKK"));
        assert!(b.contains("fg=FGG"));
        assert!(b.contains("sn="));
    }

    #[test]
    fn parse_response_equals_separator() {
        let text = "AU=https://a/BaseSystem.dmg\nAT=tokenA\nCU=https://a/BaseSystem.chunklist\nCT=tokenC\nAH=ignored\n";
        let links = parse_response(text).unwrap();
        assert_eq!(links.asset_url, "https://a/BaseSystem.dmg");
        assert_eq!(links.asset_token, "tokenA");
        assert_eq!(links.chunklist_url, "https://a/BaseSystem.chunklist");
        assert_eq!(links.chunklist_token, "tokenC");
    }

    #[test]
    fn parse_response_colon_separator() {
        let text = "AU: https://x/y.dmg\nAT: t1\nCU: https://x/y.chunklist\nCT: t2\n";
        let links = parse_response(text).unwrap();
        assert_eq!(links.asset_url, "https://x/y.dmg");
        assert_eq!(links.chunklist_token, "t2");
    }

    #[test]
    fn parse_response_missing_field_errors() {
        let text = "AU=https://a/x.dmg\nAT=tok\n"; // faltam CU/CT
        assert!(parse_response(text).is_err());
    }
}
