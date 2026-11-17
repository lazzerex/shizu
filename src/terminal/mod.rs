//! Embedded terminal spike: PTY process, emulation, and pane rendering.
//!
//! The spike uses `portable-pty` for the process side and `vt100` for the
//! emulator. `vt100` was chosen over `alacritty_terminal` because the pane
//! only needs the visible grid, cursor, colors, attributes, and alternate
//! screen. `vt100` provides those with a much smaller dependency and API.
//! Scrollback rendering and reflow are out of scope for the spike.

use std::io::{self, Read, Write};
use std::num::NonZeroU16;
use std::sync::mpsc::Sender;
use std::thread;

use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

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

/// A shell running under a PTY, with its output connected to a caller.
pub struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    parser: vt100::Parser,
    exited: bool,
}

impl Session {
    /// Spawns `command` at the given size.
    ///
    /// Output chunks are sent to `output` from a dedicated reader thread. The
    /// channel closing means the PTY reached end of file.
    pub fn spawn(size: Size, command: CommandBuilder, output: Sender<Vec<u8>>) -> io::Result<Self> {
        let (rows, cols) = size.nonzero()?;
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
            .spawn(move || read_output(&mut reader, &output))
        {
            stop_child(&mut child);
            return Err(error);
        }
        Ok(Self {
            master: pair.master,
            writer,
            child,
            parser: vt100::Parser::new(rows, cols, SCROLLBACK),
            exited: false,
        })
    }

    /// Spawns the platform default shell.
    pub fn shell(size: Size, output: Sender<Vec<u8>>) -> io::Result<Self> {
        Self::spawn(size, CommandBuilder::new_default_prog(), output)
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
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    /// Resizes both the emulator and the kernel PTY.
    pub fn resize(&mut self, size: Size) -> io::Result<()> {
        let (rows, cols) = size.nonzero()?;
        self.master.resize(size.pty()).map_err(io::Error::other)?;
        self.parser.set_size(rows, cols);
        Ok(())
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.exited = true;
        }
        Ok(status)
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        let status = self.child.wait()?;
        self.exited = true;
        Ok(status)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.exited {
            stop_child(&mut self.child);
        }
    }
}

fn stop_child(child: &mut Box<dyn Child + Send + Sync>) {
    let _ = child.kill();
    let _ = child.wait();
}

fn read_output(reader: &mut dyn Read, output: &Sender<Vec<u8>>) {
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                let Some(chunk) = buffer.get(..read) else {
                    break;
                };
                if output.send(chunk.to_vec()).is_err() {
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
    use std::sync::mpsc;
    use std::time::Duration;

    use portable_pty::CommandBuilder;

    use super::{KeyCode, KeyEvent, KeyModifiers, Session, Size, encode_key};

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
        let mut session = Session::spawn(Size { rows: 6, cols: 40 }, command, sender).unwrap();
        let mut collected = Vec::new();
        while let Ok(chunk) = receiver.recv_timeout(Duration::from_secs(5)) {
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
        let mut session = Session::spawn(Size { rows: 10, cols: 40 }, command, sender).unwrap();
        session.resize(Size { rows: 5, cols: 20 }).unwrap();
        session.write(b"go\n").unwrap();
        let mut collected = Vec::new();
        while let Ok(chunk) = receiver.recv_timeout(Duration::from_secs(5)) {
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
        let session = Session::spawn(Size { rows: 5, cols: 20 }, command, sender).unwrap();
        let pid = session.child.process_id().unwrap();
        drop(session);
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }
}
