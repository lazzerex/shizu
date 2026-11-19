use ratatui::crossterm::event::{KeyCode, KeyModifiers};

use super::{Editor, Focus};
use crate::terminal::{Pane, Size};

const MIN_VIEWPORT_HEIGHT: u16 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerminalState {
    Closed,
    Requested,
    Open(u64),
}

impl Editor {
    pub fn toggle_terminal(&mut self) {
        if self.terminal.is_some() {
            self.close_terminal();
            return;
        }
        if self.terminal_request.is_some() {
            return;
        }
        let size = self.terminal_size();
        if size.rows == 0 || size.cols == 0 {
            self.message = Some("terminal is too small".to_owned());
            return;
        }
        self.terminal_request = Some(size);
        self.focus = Focus::Terminal;
        self.bloom.clear();
    }

    pub fn take_terminal_request(&mut self) -> Option<Size> {
        self.terminal_request.take()
    }

    pub fn attach_terminal(&mut self, pane: Pane) {
        self.terminal = Some(pane);
        self.focus = Focus::Terminal;
        self.resize_terminal();
    }

    pub fn terminal_failed(&mut self, message: &str) {
        self.terminal_request = None;
        self.focus = Focus::Editor;
        self.message = Some(format!("terminal failed: {message}"));
    }

    pub fn close_terminal(&mut self) {
        self.terminal = None;
        self.terminal_request = None;
        if self.focus == Focus::Terminal {
            self.focus = Focus::Editor;
        }
    }

    pub fn terminal(&self) -> Option<&Pane> {
        self.terminal.as_ref()
    }

    pub(super) fn terminal_state(&self) -> TerminalState {
        match (&self.terminal, self.terminal_request) {
            (Some(pane), _) => TerminalState::Open(pane.id()),
            (None, Some(_)) => TerminalState::Requested,
            (None, None) => TerminalState::Closed,
        }
    }

    pub fn terminal_height(&self) -> u16 {
        if self.terminal.is_none() && self.terminal_request.is_none() {
            return 0;
        }
        self.layout_terminal_height()
    }

    pub fn feed_terminal(&mut self, id: u64, bytes: &[u8]) -> bool {
        let Some(pane) = self.terminal.as_mut() else {
            return false;
        };
        if pane.id() != id {
            return false;
        }
        let result = pane.feed(bytes);
        if let Err(error) = result {
            self.message = Some(format!("terminal write failed: {error}"));
        }
        true
    }

    pub fn terminal_closed(&mut self, id: u64) {
        if self.terminal.as_ref().is_none_or(|pane| pane.id() != id) {
            return;
        }
        self.close_terminal();
        self.message = Some("shell exited".to_owned());
    }

    pub(super) fn terminal_write_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let Some(pane) = self.terminal.as_mut() else {
            return;
        };
        if modifiers.contains(KeyModifiers::SHIFT) {
            if code == KeyCode::PageUp {
                pane.scroll_page(false);
                return;
            }
            if code == KeyCode::PageDown {
                pane.scroll_page(true);
                return;
            }
        }
        let result = pane.write_key(ratatui::crossterm::event::KeyEvent::new(code, modifiers));
        if result.is_ok() {
            pane.scroll_to_bottom();
            return;
        }
        if let Err(error) = result {
            self.message = Some(format!("terminal write failed: {error}"));
        }
    }

    pub(super) fn row_in_terminal(&self, row: u16) -> bool {
        let height = self.terminal_height();
        height > 0
            && row
                >= self
                    .viewport
                    .height
                    .saturating_sub(1)
                    .saturating_sub(height)
    }

    pub(super) fn resize_terminal(&mut self) {
        let size = self.terminal_size();
        if size.rows == 0 || size.cols == 0 {
            return;
        }
        let Some(pane) = self.terminal.as_mut() else {
            return;
        };
        if let Err(error) = pane.resize(size) {
            self.message = Some(format!("terminal resize failed: {error}"));
        }
    }

    fn layout_terminal_height(&self) -> u16 {
        if self.viewport.height < MIN_VIEWPORT_HEIGHT {
            return 0;
        }
        (self.viewport.height / 3)
            .clamp(5, 16)
            .min(self.viewport.height.saturating_sub(5))
    }

    fn terminal_size(&self) -> Size {
        Size {
            rows: self.layout_terminal_height().saturating_sub(1),
            cols: self.viewport.width,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Editor;
    use crate::buffer::Buffer;
    use crate::command::Command;
    use crate::editor::Focus;
    use crate::terminal::{Pane, Size};
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    fn editor() -> Editor {
        let mut editor = Editor::new(Buffer::from_text("abc"));
        editor.resize(40, 12);
        editor
    }

    fn pane() -> Pane {
        Pane::for_test(Size { rows: 3, cols: 40 })
    }

    #[test]
    fn toggling_the_terminal_opens_a_reserved_bottom_pane() {
        let mut editor = editor();

        assert!(editor.apply(Command::ToggleTerminal));

        assert_eq!(editor.focus(), Focus::Terminal);
        assert_eq!(editor.terminal_height(), 5);
        assert!(editor.take_terminal_request().is_some());
    }

    #[test]
    fn attaching_and_closing_the_terminal_restores_the_editor_focus() {
        let mut editor = editor();
        editor.toggle_terminal();
        editor.attach_terminal(pane());
        assert_eq!(editor.focus(), Focus::Terminal);
        assert!(editor.terminal().is_some());

        assert!(editor.apply(Command::ToggleTerminal));

        assert_eq!(editor.focus(), Focus::Editor);
        assert!(editor.terminal().is_none());
        assert_eq!(editor.terminal_height(), 0);
    }

    #[test]
    fn output_from_the_current_session_is_fed_to_the_emulator() {
        let mut editor = editor();
        editor.attach_terminal(pane());
        let id = editor.terminal().expect("terminal pane").id();

        assert!(editor.feed_terminal(id, b"hello"));

        assert!(
            editor
                .terminal()
                .expect("terminal pane")
                .screen()
                .contents()
                .contains("hello")
        );
    }

    #[test]
    fn output_from_an_old_session_is_ignored() {
        let mut editor = editor();
        editor.attach_terminal(pane());
        let id = editor.terminal().expect("terminal pane").id();

        assert!(!editor.feed_terminal(id + 1, b"stale"));
        assert!(
            !editor
                .terminal()
                .expect("terminal pane")
                .screen()
                .contents()
                .contains("stale")
        );
    }

    #[test]
    fn closing_the_terminal_from_the_editor_reports_a_change() {
        let mut editor = editor();
        editor.attach_terminal(pane());
        editor.apply(Command::Click { column: 2, row: 1 });
        assert_eq!(editor.focus(), Focus::Editor);

        assert!(editor.apply(Command::ToggleTerminal));

        assert!(editor.terminal().is_none());
        assert_eq!(editor.focus(), Focus::Editor);
    }

    #[test]
    fn closing_the_shell_closes_the_pane() {
        let mut editor = editor();
        editor.attach_terminal(pane());
        let id = editor.terminal().expect("terminal pane").id();

        editor.terminal_closed(id);

        assert!(editor.terminal().is_none());
        assert_eq!(editor.focus(), Focus::Editor);
        assert_eq!(editor.message(), Some("shell exited"));
    }

    #[test]
    fn terminal_keys_do_not_edit_the_buffer() {
        let mut editor = editor();
        editor.attach_terminal(pane());

        editor.apply(Command::TerminalKey(
            ratatui::crossterm::event::KeyCode::Char('x'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        ));

        assert_eq!(editor.buffer().text(), "abc");
    }

    #[test]
    fn clicking_the_terminal_pane_restores_its_focus() {
        let mut editor = editor();
        editor.attach_terminal(pane());
        editor.apply(Command::OpenFind);
        assert_eq!(editor.focus(), Focus::Find);

        editor.apply(Command::Click { column: 2, row: 8 });

        assert_eq!(editor.focus(), Focus::Terminal);
    }

    #[test]
    fn tiny_viewports_refuse_to_open_the_terminal() {
        let mut editor = editor();
        editor.resize(20, 6);

        assert!(editor.apply(Command::ToggleTerminal));

        assert_eq!(editor.focus(), Focus::Editor);
        assert_eq!(editor.message(), Some("terminal is too small"));
    }

    fn scrolled_editor() -> Editor {
        let mut editor = editor();
        let mut pane = pane();
        for line in 0..8 {
            pane.feed(format!("line{line}\r\n").as_bytes())
                .expect("feed test output");
        }
        editor.attach_terminal(pane);
        editor
    }

    #[test]
    fn shift_page_up_scrolls_the_terminal_view() {
        let mut editor = scrolled_editor();

        assert!(editor.apply(Command::TerminalKey(KeyCode::PageUp, KeyModifiers::SHIFT,)));

        assert!(editor.terminal().expect("terminal pane").scrollback() > 0);
        assert_eq!(editor.buffer().text(), "abc");
    }

    #[test]
    fn the_wheel_scrolls_the_terminal_when_it_is_focused() {
        let mut editor = scrolled_editor();

        assert!(editor.apply(Command::ScrollLines(-3)));

        assert!(editor.terminal().expect("terminal pane").scrollback() > 0);
    }

    #[test]
    fn typing_returns_the_terminal_to_the_bottom() {
        let mut editor = scrolled_editor();
        editor.apply(Command::ScrollLines(-3));
        assert!(editor.terminal().expect("terminal pane").scrollback() > 0);

        assert!(editor.apply(Command::TerminalKey(KeyCode::Char('x'), KeyModifiers::NONE,)));

        assert_eq!(editor.terminal().expect("terminal pane").scrollback(), 0);
    }

    #[test]
    fn the_wheel_scrolls_the_editor_when_the_terminal_is_unfocused() {
        let mut editor = scrolled_editor();
        editor.apply(Command::Click { column: 2, row: 1 });
        assert_eq!(editor.focus(), Focus::Editor);

        editor.apply(Command::ScrollLines(-3));

        assert_eq!(editor.terminal().expect("terminal pane").scrollback(), 0);
    }
}
