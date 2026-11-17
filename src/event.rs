use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use ratatui::crossterm::event::{self as terminal, Event as TerminalEvent, KeyEvent, MouseEvent};

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    PtyOutput(u64, Vec<u8>),
    PtyClosed(u64),
    Tick,
}

pub fn channel() -> io::Result<(Sender<Event>, Receiver<Event>)> {
    let (sender, receiver) = mpsc::channel();
    let events = sender.clone();
    thread::Builder::new()
        .name("shizu-input".to_owned())
        .spawn(move || {
            while let Ok(event) = terminal::read() {
                let event = match event {
                    TerminalEvent::Key(key) => Event::Key(key),
                    TerminalEvent::Mouse(mouse) => Event::Mouse(mouse),
                    TerminalEvent::Resize(width, height) => Event::Resize(width, height),
                    _ => continue,
                };
                if sender.send(event).is_err() {
                    break;
                }
            }
        })?;
    Ok((events, receiver))
}
