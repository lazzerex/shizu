use std::io::{self, stdout};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Instant;

use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{cursor::Show, execute};
use shizu::buffer::Buffer;
use shizu::clipboard::Clipboard;
use shizu::command::Command;
use shizu::config::{self, Config};
use shizu::editor::Editor;
use shizu::theme::{ColorDepth, Theme};
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

fn load_config() -> (Config, Option<String>) {
    let Some(path) = config::default_path() else {
        return (Config::default(), None);
    };
    match Config::load(&path) {
        Ok(config) => (config, None),
        Err(error) => (Config::default(), Some(error.to_string())),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let buffer = match std::env::args_os().nth(1) {
        Some(path) => Buffer::open(path)?,
        None => Buffer::new(),
    };
    let (config, mut config_message) = load_config();
    let depth = ColorDepth::detect();
    let theme = if let Some(theme) = Theme::named(&config.theme, depth) {
        theme
    } else {
        config_message.get_or_insert_with(|| format!("unknown theme: {}", config.theme));
        Theme::sakura(depth)
    };
    let mut editor = Editor::new(buffer);
    editor.set_theme(theme);
    editor.set_tab_width(config.tab_width);
    editor.set_animation_mode(config.animation);
    if let Some(message) = config_message {
        editor.set_message(message);
    }

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
    let mut clipboard = Clipboard::new();
    loop {
        let wait_now = Instant::now();
        let event = match editor.next_wait(wait_now) {
            Some(wait) => match events.recv_timeout(wait) {
                Ok(event) => event,
                Err(RecvTimeoutError::Timeout) => Event::Tick,
                Err(RecvTimeoutError::Disconnected) => break,
            },
            None => match events.recv() {
                Ok(event) => event,
                Err(_) => break,
            },
        };
        let now = Instant::now();
        let mut changed = match event {
            Event::Key(key) => {
                let was_animating = editor.animation_visible(now);
                editor.note_input(now);
                let command = keymap::resolve(&key, editor.focus());
                if matches!(command, Some(Command::Paste))
                    && let Some(text) = clipboard.get_text()
                {
                    editor.set_clipboard_text(text);
                }
                command.is_some_and(|command| editor.apply(command)) || was_animating
            }
            Event::Mouse(mouse) => {
                let was_animating = editor.animation_visible(now);
                editor.note_input(now);
                keymap::resolve_mouse(&mouse).is_some_and(|command| editor.apply(command))
                    || was_animating
            }
            Event::Resize(width, height) => {
                editor.resize(width, height);
                true
            }
            Event::Tick => editor.tick(now),
        };
        if let Some(text) = editor.take_clipboard_request()
            && !clipboard.set_text(&text)
        {
            editor.set_message("clipboard unavailable");
            changed = true;
        }
        if editor.should_quit() {
            break;
        }
        if changed {
            terminal.draw(|frame| ui::draw(frame, &editor))?;
        }
    }

    Ok(())
}
