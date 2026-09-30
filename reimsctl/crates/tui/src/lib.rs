//! reimsctl-tui — console principal do appliance.
//!
//! Por ora um menu simples em stdout (placeholder). Na fase-1 vira uma TUI de
//! tela cheia com `ratatui`/`crossterm` (autologin no tty1).

use anyhow::Result;
use reimsctl_core::release::MacosRelease;

/// Desenha o menu principal (placeholder textual).
pub fn render_main_menu() -> Result<()> {
    println!("┌─ OSX-REIMS-OS ───────────────────────────────┐");
    println!("│ 1) Baixar macOS (Apple)                       │");
    println!("│ 2) Criar VM                                   │");
    println!("│ 3) Rodar VM                                   │");
    println!("│ 4) Atualizações                               │");
    println!("│ 5) Sair                                       │");
    println!("└───────────────────────────────────────────────┘");
    Ok(())
}

/// Lista as versões de macOS suportadas (para o passo "Baixar macOS").
pub fn print_releases() {
    for (i, r) in MacosRelease::ALL.iter().enumerate() {
        println!("  {}) macOS {} ({})", i + 1, r.major(), r.code_name());
    }
}
