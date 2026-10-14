//! Primitivas do console de texto: limpar a tela, cores, uma tecla, uma linha.

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};

pub fn clear() {
    let mut out = io::stdout();
    let _ = execute!(
        out,
        Clear(ClearType::All),
        Clear(ClearType::Purge),
        cursor::MoveTo(0, 0)
    );
}

fn paint(color: Option<Color>, bold: bool, dim: bool, s: &str) {
    let mut out = io::stdout();
    if bold {
        let _ = execute!(out, SetAttribute(Attribute::Bold));
    }
    if dim {
        let _ = execute!(out, SetAttribute(Attribute::Dim));
    }
    if let Some(c) = color {
        let _ = execute!(out, SetForegroundColor(c));
    }
    print!("{s}");
    let _ = execute!(out, ResetColor, SetAttribute(Attribute::Reset));
}

pub fn bold(s: &str) {
    paint(None, true, false, s);
}
pub fn accent(s: &str) {
    paint(Some(Color::Cyan), true, false, s);
}
pub fn good(s: &str) {
    paint(Some(Color::Green), false, false, s);
}
pub fn warn(s: &str) {
    paint(Some(Color::Yellow), false, false, s);
}
pub fn bad(s: &str) {
    paint(Some(Color::Red), false, false, s);
}
pub fn dim(s: &str) {
    paint(None, false, true, s);
}

pub fn rule() {
    dim(&format!("{}\n", "─".repeat(64)));
}

/// Uma opção de menu: `  1  Create new VM  (detalhe)`.
pub fn item(key: &str, label: &str, detail: &str, enabled: bool) {
    print!("  ");
    if enabled {
        accent(key);
        print!("  {label}");
    } else {
        dim(&format!("{key}  {label}"));
    }
    if !detail.is_empty() {
        dim(&format!("  ({detail})"));
    }
    println!();
}

/// Espera uma tecla e a devolve em minúscula; Enter é `'\n'`, Esc é `'\x1b'`.
/// Ctrl+C devolve `'\x03'`.
pub fn key(prompt: &str) -> char {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let raw = terminal::enable_raw_mode().is_ok();
    let k = loop {
        match event::read() {
            Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => match k.code {
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break '\x03',
                KeyCode::Char(c) => break c.to_ascii_lowercase(),
                KeyCode::Enter => break '\n',
                KeyCode::Esc => break '\x1b',
                _ => {}
            },
            Ok(_) => {}
            Err(_) => break '\x1b',
        }
    };
    if raw {
        let _ = terminal::disable_raw_mode();
    }
    if k.is_ascii_graphic() {
        println!("{k}");
    } else {
        println!();
    }
    k
}

/// Uma linha digitada; vazio devolve o padrão.
pub fn ask(prompt: &str, default: &str) -> String {
    if default.is_empty() {
        print!("{prompt}: ");
    } else {
        print!("{prompt} [{default}]: ");
    }
    let _ = io::stdout().flush();
    let mut s = String::new();
    if io::stdin().read_line(&mut s).is_err() {
        return default.to_string();
    }
    let s = s.trim();
    if s.is_empty() {
        default.to_string()
    } else {
        s.to_string()
    }
}

/// Sim/não; Enter devolve o padrão.
pub fn confirm(prompt: &str, default_yes: bool) -> bool {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    loop {
        match key(&format!("{prompt} {hint} ")) {
            'y' | 's' => return true,
            'n' | '\x1b' | '\x03' => return false,
            '\n' => return default_yes,
            _ => {}
        }
    }
}

pub fn pause() {
    key("\nPress any key to go back. ");
}
