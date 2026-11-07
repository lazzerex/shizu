use std::io::{self, stdout};

use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{cursor::Show, execute};
use shizu::buffer::Buffer;
use shizu::editor::Editor;
use shizu::{Event, event, keymap, ui};

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        let guard = Self;
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore();
    }
}

fn restore() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture, Show)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let buffer = match std::env::args_os().nth(1) {
        Some(path) => Buffer::open(path)?,
        None => Buffer::new(),
    };
    let mut editor = Editor::new(buffer);

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        hook(info);
    }));

    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let size = terminal.backend().size()?;
    editor.resize(size.width, size.height);
    terminal.draw(|frame| ui::draw(frame, &editor))?;

    let events = event::channel()?;
    while let Ok(event) = events.recv() {
        let changed = match event {
            Event::Key(key) => keymap::resolve(&key).is_some_and(|command| editor.apply(command)),
            Event::Mouse(mouse) => {
                keymap::resolve_mouse(&mouse).is_some_and(|command| editor.apply(command))
            }
            Event::Resize(width, height) => {
                editor.resize(width, height);
                true
            }
        };
        if editor.should_quit() {
            break;
        }
        if changed {
            terminal.draw(|frame| ui::draw(frame, &editor))?;
        }
    }

    Ok(())
}
