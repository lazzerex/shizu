//! Embedded terminal spike.
//!
//! Runs a shell through `portable-pty`, renders it with `vt100`, forwards
//! keys, and resizes the PTY when the pane changes size.
//!
//! Run with `cargo run --example terminal_spike`. Ctrl+Q closes the spike.

use std::io::{self, stdout};
use std::sync::mpsc;
use std::thread;

use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{
    self as terminal_event, Event as TerminalEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{cursor::Show, execute};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use shizu::terminal::{self, Session, Size};

const FOOTER_HEIGHT: u16 = 2;
const BORDER_HEIGHT: u16 = 2;
const BORDER_WIDTH: u16 = 2;

enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Output(Vec<u8>),
    Closed,
}

struct Guard;

impl Guard {
    fn enter() -> io::Result<Self> {
        let guard = Self;
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        Ok(guard)
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        hook(info);
    }));

    let _guard = Guard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let area = terminal.backend().size()?;
    let size =
        pane_size(area.width, area.height).ok_or("terminal is too small for the terminal spike")?;

    let (output_tx, output_rx) = mpsc::channel();
    let mut session = Session::shell(size, output_tx)?;
    let (event_tx, event_rx) = mpsc::channel();

    let output_events = event_tx.clone();
    thread::Builder::new()
        .name("shizu-pty-output".to_owned())
        .spawn(move || {
            while let Ok(bytes) = output_rx.recv() {
                if output_events.send(AppEvent::Output(bytes)).is_err() {
                    return;
                }
            }
            let _ = output_events.send(AppEvent::Closed);
        })?;

    thread::Builder::new()
        .name("shizu-spike-input".to_owned())
        .spawn(move || {
            while let Ok(event) = terminal_event::read() {
                let event = match event {
                    TerminalEvent::Key(key) if key.kind != KeyEventKind::Release => {
                        AppEvent::Key(key)
                    }
                    TerminalEvent::Resize(width, height) => AppEvent::Resize(width, height),
                    _ => continue,
                };
                if event_tx.send(event).is_err() {
                    break;
                }
            }
        })?;

    let mut message = None;
    terminal.draw(|frame| draw(frame, &session, message.as_deref()))?;
    while let Ok(event) = event_rx.recv() {
        let mut changed = true;
        match event {
            AppEvent::Key(key) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
                    break;
                }
                if message.is_some() {
                    break;
                }
                session.write_key(key)?;
                changed = false;
            }
            AppEvent::Resize(width, height) => {
                if let Some(size) = pane_size(width, height) {
                    session.resize(size)?;
                }
            }
            AppEvent::Output(bytes) => session.feed(&bytes),
            AppEvent::Closed => {
                let status = session.wait()?;
                message = Some(format!("shell exited ({status}); press any key to close"));
            }
        }
        if changed {
            terminal.draw(|frame| draw(frame, &session, message.as_deref()))?;
        }
    }

    Ok(())
}

fn restore() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, Show)?;
    Ok(())
}

fn pane_size(width: u16, height: u16) -> Option<Size> {
    let rows = height.checked_sub(FOOTER_HEIGHT + BORDER_HEIGHT)?;
    let cols = width.checked_sub(BORDER_WIDTH)?;
    (rows > 0 && cols > 0).then_some(Size { rows, cols })
}

fn draw(frame: &mut ratatui::Frame, session: &Session, message: Option<&str>) {
    let area = frame.area();
    let [pane, footer] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(FOOTER_HEIGHT)]).areas(area);
    let (cols, rows) = session.screen().size();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().add_modifier(Modifier::DIM))
        .title(format!(
            " terminal · vt100 · {}x{} ",
            cols.get(),
            rows.get()
        ));
    let inner = block.inner(pane);
    frame.render_widget(block, pane);
    if let Some((x, y)) = terminal::draw(session.screen(), frame.buffer_mut(), inner) {
        frame.set_cursor_position((x, y));
    }
    let status = message.unwrap_or("Ctrl+Q quit · keys and pane resizes go to the shell");
    let text = Paragraph::new(vec![
        Line::from(status),
        Line::from("shizu terminal spike · portable-pty + vt100"),
    ])
    .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(text, footer);
}
