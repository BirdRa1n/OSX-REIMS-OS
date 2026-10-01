//! reimsctl-tui — console principal do appliance (ratatui).
//!
//! `run()` desenha um menu de tela cheia, trata navegação (↑/↓, Enter, q) e
//! devolve a [`Action`] escolhida; o binário `reimsctl` executa a ação e volta
//! a chamar `run()`. Ações que precisam de parâmetros são tratadas pelo chamador
//! (o terminal já foi restaurado quando `run` retorna).

use anyhow::Result;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, Terminal};
use reimsctl_core::release::MacosRelease;
use std::io::stdout;

/// Ação escolhida pelo usuário no menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    FetchMacos,
    CreateVm,
    ProvisionVm,
    Update,
    StartVm(String),
    Quit,
}

struct Entry {
    label: String,
    action: Action,
}

/// Monta os itens do menu, incluindo uma entrada "Rodar VM" por VM existente.
fn build_entries(vms: &[String]) -> Vec<Entry> {
    let mut e = vec![
        Entry {
            label: "⬇  Baixar macOS".into(),
            action: Action::FetchMacos,
        },
        Entry {
            label: "✚  Criar VM".into(),
            action: Action::CreateVm,
        },
        Entry {
            label: "⚙  Provisionar VM (macOS + OpenCore + disco)".into(),
            action: Action::ProvisionVm,
        },
        Entry {
            label: "⟳  Atualizações".into(),
            action: Action::Update,
        },
    ];
    for vm in vms {
        e.push(Entry {
            label: format!("▶  Rodar VM: {vm}"),
            action: Action::StartVm(vm.clone()),
        });
    }
    e.push(Entry {
        label: "⏻  Sair".into(),
        action: Action::Quit,
    });
    e
}

/// Roda uma sessão do menu e retorna a ação escolhida.
pub fn run(vms: &[String]) -> Result<Action> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;

    let res = run_app(&mut terminal, vms);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, vms: &[String]) -> Result<Action> {
    let entries = build_entries(vms);
    let mut state = ListState::default();
    state.select(Some(0));

    loop {
        terminal.draw(|f| ui(f, &entries, &mut state))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(Action::Quit),
                KeyCode::Down | KeyCode::Char('j') => move_sel(&mut state, entries.len(), 1),
                KeyCode::Up | KeyCode::Char('k') => move_sel(&mut state, entries.len(), -1),
                KeyCode::Enter => {
                    let i = state.selected().unwrap_or(0);
                    return Ok(entries[i].action.clone());
                }
                _ => {}
            }
        }
    }
}

fn move_sel(state: &mut ListState, len: usize, delta: isize) {
    if len == 0 {
        return;
    }
    let cur = state.selected().unwrap_or(0) as isize;
    let next = (cur + delta).rem_euclid(len as isize);
    state.select(Some(next as usize));
}

fn ui(f: &mut Frame, entries: &[Entry], state: &mut ListState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    let title = Paragraph::new(Line::from("OSX-REIMS-OS"))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = entries
        .iter()
        .map(|e| ListItem::new(e.label.clone()))
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" menu "))
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("➤ ");
    f.render_stateful_widget(list, chunks[1], state);

    let help = Paragraph::new(Line::from("↑/↓ navegar · Enter selecionar · q sair"))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);
}

/// Lista as versões de macOS suportadas (usado em prompts fora da TUI).
pub fn print_releases() {
    for (i, r) in MacosRelease::ALL.iter().enumerate() {
        println!("  {}) macOS {} ({})", i + 1, r.major(), r.code_name());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_include_vms_and_quit_last() {
        let e = build_entries(&["alpha".into(), "beta".into()]);
        // 4 fixas + 2 VMs + Sair
        assert_eq!(e.len(), 7);
        assert_eq!(e[0].action, Action::FetchMacos);
        assert_eq!(e[4].action, Action::StartVm("alpha".into()));
        assert_eq!(e[5].action, Action::StartVm("beta".into()));
        assert_eq!(e.last().unwrap().action, Action::Quit);
    }

    #[test]
    fn no_vms_has_no_start_entries() {
        let e = build_entries(&[]);
        assert_eq!(e.len(), 5); // 4 fixas + Sair
        assert!(!e.iter().any(|x| matches!(x.action, Action::StartVm(_))));
    }

    #[test]
    fn move_sel_wraps() {
        let mut s = ListState::default();
        s.select(Some(0));
        move_sel(&mut s, 3, -1);
        assert_eq!(s.selected(), Some(2));
        move_sel(&mut s, 3, 1);
        assert_eq!(s.selected(), Some(0));
    }
}
