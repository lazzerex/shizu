//! Embedded terminal spike: PTY process, emulation, and pane rendering.
//!
//! The spike uses `portable-pty` for the process side and `vt100` for the
//! emulator. `vt100` was chosen over `alacritty_terminal` because the pane
//! only needs the visible grid, cursor, colors, attributes, and alternate
//! screen. `vt100` provides those with a much smaller dependency and API.
//! Scrollback rendering and reflow are out of scope for the spike.

use std::io::{self, Read, Write};
use std::num::NonZeroU16;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::thread;

use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::event::Event;

mod view;

pub use view::draw;

const SCROLLBACK: usize = 1000;

/// Character-cell dimensions of a terminal pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub rows: u16,
    pub cols: u16,
}

impl Size {
    fn nonzero(self) -> io::Result<(NonZeroU16, NonZeroU16)> {
        let rows = NonZeroU16::new(self.rows).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "terminal rows must not be zero",
            )
        })?;
        let cols = NonZeroU16::new(self.cols).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "terminal columns must not be zero",
            )
        })?;
        Ok((rows, cols))
    }

    const fn pty(self) -> PtySize {
        PtySize {
            rows: self.rows,
            cols: self.cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

pub struct Pane {
    id: u64,
    session: Option<Session>,
    parser: vt100::Parser,
}

impl Pane {
    /// Spawns the platform default shell at the given size.
    pub fn shell(size: Size, output: Sender<Event>) -> io::Result<Self> {
        Self::spawn(size, CommandBuilder::new_default_prog(), output)
    }

    fn spawn(size: Size, command: CommandBuilder, output: Sender<Event>) -> io::Result<Self> {
        let id = next_id();
        let session = Session::spawn(size, command, output, id)?;
        let (rows, cols) = size.nonzero()?;
        Ok(Self {
            id,
            session: Some(session),
            parser: vt100::Parser::new(rows, cols, SCROLLBACK),
        })
    }

    #[cfg(test)]
    pub(crate) fn for_test(size: Size) -> Self {
        let (rows, cols) = size.nonzero().expect("valid terminal size");
        Self {
            id: next_id(),
            session: None,
            parser: vt100::Parser::new(rows, cols, SCROLLBACK),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// Feeds PTY output into the emulator.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// Forwards one key to the shell.
    pub fn write_key(&mut self, key: KeyEvent) -> io::Result<()> {
        let Some(bytes) = encode_key(&key) else {
            return Ok(());
        };
        self.write(&bytes)
    }

    pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let Some(session) = self.session.as_mut() else {
            return Ok(());
        };
        session.write(bytes)
    }

    /// Resizes both the emulator and the kernel PTY.
    pub fn resize(&mut self, size: Size) -> io::Result<()> {
        let (rows, cols) = size.nonzero()?;
        if let Some(session) = self.session.as_mut() {
            session.resize(size)?;
        }
        self.parser.set_size(rows, cols);
        Ok(())
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        let Some(session) = self.session.as_mut() else {
            return Err(io::Error::other("terminal pane has no process"));
        };
        session.wait()
    }

    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn process_id(&self) -> Option<u32> {
        self.session
            .as_ref()
            .and_then(|session| session.child.process_id())
    }
}

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

impl Session {
    fn spawn(
        size: Size,
        command: CommandBuilder,
        output: Sender<Event>,
        id: u64,
    ) -> io::Result<Self> {
        let pair = native_pty_system()
            .openpty(size.pty())
            .map_err(io::Error::other)?;
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(io::Error::other)?;
        drop(pair.slave);
        let mut reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(error) => {
                stop_child(&mut child);
                return Err(io::Error::other(error));
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(error) => {
                stop_child(&mut child);
                return Err(io::Error::other(error));
            }
        };
        if let Err(error) = thread::Builder::new()
            .name("shizu-pty".to_owned())
            .spawn(move || read_output(&mut reader, &output, id))
        {
            stop_child(&mut child);
            return Err(error);
        }
        Ok(Self {
            master: pair.master,
            writer,
            child,
        })
    }

    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    fn resize(&mut self, size: Size) -> io::Result<()> {
        self.master.resize(size.pty()).map_err(io::Error::other)
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        self.child.wait()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        stop_child(&mut self.child);
    }
}

fn next_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn stop_child(child: &mut Box<dyn Child + Send + Sync>) {
    let _ = child.kill();
    let _ = child.wait();
}

fn read_output(reader: &mut dyn Read, output: &Sender<Event>, id: u64) {
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                let _ = output.send(Event::PtyClosed(id));
                break;
            }
            Ok(read) => {
                let Some(chunk) = buffer.get(..read) else {
                    break;
                };
                if output.send(Event::PtyOutput(id, chunk.to_vec())).is_err() {
                    break;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
}

fn encode_key(key: &KeyEvent) -> Option<Vec<u8>> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let mut bytes = Vec::new();
    match key.code {
        KeyCode::Char(character) if control => {
            if alt {
                bytes.push(0x1b);
            }
            bytes.push(control_byte(character)?);
        }
        KeyCode::Char(character) => {
            if alt {
                bytes.push(0x1b);
            }
            let mut encoded = [0u8; 4];
            bytes.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
        }
        KeyCode::Enter => bytes.push(b'\r'),
        KeyCode::Tab => bytes.push(b'\t'),
        KeyCode::BackTab => bytes.extend_from_slice(b"\x1b[Z"),
        KeyCode::Backspace => bytes.push(0x7f),
        KeyCode::Esc => bytes.push(0x1b),
        KeyCode::Up => bytes.extend_from_slice(b"\x1b[A"),
        KeyCode::Down => bytes.extend_from_slice(b"\x1b[B"),
        KeyCode::Right => bytes.extend_from_slice(b"\x1b[C"),
        KeyCode::Left => bytes.extend_from_slice(b"\x1b[D"),
        KeyCode::Home => bytes.extend_from_slice(b"\x1b[H"),
        KeyCode::End => bytes.extend_from_slice(b"\x1b[F"),
        KeyCode::Insert => bytes.extend_from_slice(b"\x1b[2~"),
        KeyCode::Delete => bytes.extend_from_slice(b"\x1b[3~"),
        KeyCode::PageUp => bytes.extend_from_slice(b"\x1b[5~"),
        KeyCode::PageDown => bytes.extend_from_slice(b"\x1b[6~"),
        KeyCode::F(number) => bytes.extend_from_slice(function_key(number)?),
        _ => return None,
    }
    Some(bytes)
}

fn control_byte(character: char) -> Option<u8> {
    match character {
        '@' | ' ' => Some(0x00),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' => Some(0x1e),
        '_' => Some(0x1f),
        '?' => Some(0x7f),
        _ => {
            let byte = u8::try_from(character.to_ascii_lowercase()).ok()?;
            byte.is_ascii_lowercase().then_some(byte - b'a' + 1)
        }
    }
}

fn function_key(number: u8) -> Option<&'static [u8]> {
    match number {
        1 => Some(b"\x1bOP"),
        2 => Some(b"\x1bOQ"),
        3 => Some(b"\x1bOR"),
        4 => Some(b"\x1bOS"),
        5 => Some(b"\x1b[15~"),
        6 => Some(b"\x1b[17~"),
        7 => Some(b"\x1b[18~"),
        8 => Some(b"\x1b[19~"),
        9 => Some(b"\x1b[20~"),
        10 => Some(b"\x1b[21~"),
        11 => Some(b"\x1b[23~"),
        12 => Some(b"\x1b[24~"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::sync::mpsc;
    #[cfg(unix)]
    use std::time::Duration;

    #[cfg(unix)]
    use portable_pty::CommandBuilder;

    use super::{KeyCode, KeyEvent, KeyModifiers, encode_key};
    #[cfg(unix)]
    use super::{Pane, Size};
    #[cfg(unix)]
    use crate::event::Event;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn encodes_text_and_control_keys() {
        assert_eq!(
            encode_key(&key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(b"a".to_vec())
        );
        assert_eq!(
            encode_key(&key(KeyCode::Char('C'), KeyModifiers::CONTROL)),
            Some(vec![0x03])
        );
        assert_eq!(
            encode_key(&key(KeyCode::Char('x'), KeyModifiers::ALT)),
            Some(b"\x1bx".to_vec())
        );
        assert_eq!(
            encode_key(&key(KeyCode::Esc, KeyModifiers::NONE)),
            Some(vec![0x1b])
        );
        assert_eq!(
            encode_key(&key(KeyCode::Backspace, KeyModifiers::NONE)),
            Some(vec![0x7f])
        );
    }

    #[test]
    fn encodes_navigation_and_function_keys() {
        assert_eq!(
            encode_key(&key(KeyCode::Up, KeyModifiers::NONE)),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            encode_key(&key(KeyCode::Delete, KeyModifiers::NONE)),
            Some(b"\x1b[3~".to_vec())
        );
        assert_eq!(
            encode_key(&key(KeyCode::F(5), KeyModifiers::NONE)),
            Some(b"\x1b[15~".to_vec())
        );
        assert_eq!(
            encode_key(&key(KeyCode::BackTab, KeyModifiers::SHIFT)),
            Some(b"\x1b[Z".to_vec())
        );
    }

    #[test]
    fn ignores_unsupported_keys() {
        assert_eq!(encode_key(&key(KeyCode::Null, KeyModifiers::NONE)), None);
        assert_eq!(
            encode_key(&key(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            Some(vec![0x1b, 0x01])
        );
    }

    #[cfg(unix)]
    #[test]
    fn runs_a_shell_and_collects_its_output() {
        let (sender, receiver) = mpsc::channel();
        let mut command = CommandBuilder::new("sh");
        command.arg("-c");
        command.arg("printf 'hello from pty\\n'");
        let mut session = Pane::spawn(Size { rows: 6, cols: 40 }, command, sender).unwrap();
        let mut collected = Vec::new();
        while let Ok(Event::PtyOutput(_, chunk)) = receiver.recv_timeout(Duration::from_secs(5)) {
            collected.extend_from_slice(&chunk);
            session.feed(&chunk);
            if collected
                .windows(b"hello from pty".len())
                .any(|window| window == b"hello from pty")
            {
                break;
            }
        }
        assert!(session.screen().contents().contains("hello from pty"));
        let status = session.wait().unwrap();
        assert!(status.success());
    }

    #[cfg(unix)]
    #[test]
    fn resizes_the_shell_with_the_pane() {
        let (sender, receiver) = mpsc::channel();
        let mut command = CommandBuilder::new("sh");
        command.arg("-c");
        command.arg("read line; stty size");
        let mut session = Pane::spawn(Size { rows: 10, cols: 40 }, command, sender).unwrap();
        session.resize(Size { rows: 5, cols: 20 }).unwrap();
        session.write(b"go\n").unwrap();
        let mut collected = Vec::new();
        while let Ok(Event::PtyOutput(_, chunk)) = receiver.recv_timeout(Duration::from_secs(5)) {
            collected.extend_from_slice(&chunk);
            session.feed(&chunk);
            if collected
                .windows(b"5 20".len())
                .any(|window| window == b"5 20")
            {
                break;
            }
        }
        assert!(session.screen().contents().contains("5 20"));
        let status = session.wait().unwrap();
        assert!(status.success());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn dropping_the_session_kills_and_reaps_the_shell() {
        let (sender, _receiver) = mpsc::channel();
        let mut command = CommandBuilder::new("sh");
        command.arg("-c");
        command.arg("sleep 30");
        let session = Pane::spawn(Size { rows: 5, cols: 20 }, command, sender).unwrap();
        let pid = session.process_id().unwrap();
        drop(session);
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }
}
