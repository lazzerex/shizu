use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::command::Command;

const TABLE: &[(KeyCode, KeyModifiers, Command)] = &[
    (
        KeyCode::Left,
        KeyModifiers::NONE,
        Command::MoveLeft { select: false },
    ),
    (
        KeyCode::Left,
        KeyModifiers::SHIFT,
        Command::MoveLeft { select: true },
    ),
    (
        KeyCode::Right,
        KeyModifiers::NONE,
        Command::MoveRight { select: false },
    ),
    (
        KeyCode::Right,
        KeyModifiers::SHIFT,
        Command::MoveRight { select: true },
    ),
    (
        KeyCode::Up,
        KeyModifiers::NONE,
        Command::MoveUp { select: false },
    ),
    (
        KeyCode::Up,
        KeyModifiers::SHIFT,
        Command::MoveUp { select: true },
    ),
    (
        KeyCode::Down,
        KeyModifiers::NONE,
        Command::MoveDown { select: false },
    ),
    (
        KeyCode::Down,
        KeyModifiers::SHIFT,
        Command::MoveDown { select: true },
    ),
    (
        KeyCode::Home,
        KeyModifiers::NONE,
        Command::Home { select: false },
    ),
    (
        KeyCode::Home,
        KeyModifiers::SHIFT,
        Command::Home { select: true },
    ),
    (
        KeyCode::End,
        KeyModifiers::NONE,
        Command::End { select: false },
    ),
    (
        KeyCode::End,
        KeyModifiers::SHIFT,
        Command::End { select: true },
    ),
    (
        KeyCode::PageUp,
        KeyModifiers::NONE,
        Command::PageUp { select: false },
    ),
    (
        KeyCode::PageUp,
        KeyModifiers::SHIFT,
        Command::PageUp { select: true },
    ),
    (
        KeyCode::PageDown,
        KeyModifiers::NONE,
        Command::PageDown { select: false },
    ),
    (
        KeyCode::PageDown,
        KeyModifiers::SHIFT,
        Command::PageDown { select: true },
    ),
    (KeyCode::Up, KeyModifiers::ALT, Command::MoveLineUp),
    (KeyCode::Down, KeyModifiers::ALT, Command::MoveLineDown),
    (KeyCode::Backspace, KeyModifiers::NONE, Command::Backspace),
    (KeyCode::Delete, KeyModifiers::NONE, Command::Delete),
    (KeyCode::Enter, KeyModifiers::NONE, Command::Insert('\n')),
    (KeyCode::Tab, KeyModifiers::NONE, Command::Insert('\t')),
    (KeyCode::Esc, KeyModifiers::NONE, Command::Cancel),
    (KeyCode::Char('s'), KeyModifiers::CONTROL, Command::Save),
    (KeyCode::Char('q'), KeyModifiers::CONTROL, Command::Quit),
    (
        KeyCode::Char('a'),
        KeyModifiers::CONTROL,
        Command::SelectAll,
    ),
    (KeyCode::Char('x'), KeyModifiers::CONTROL, Command::Cut),
    (KeyCode::Char('c'), KeyModifiers::CONTROL, Command::Copy),
    (KeyCode::Char('v'), KeyModifiers::CONTROL, Command::Paste),
    (KeyCode::Char('z'), KeyModifiers::CONTROL, Command::Undo),
    (KeyCode::Char('y'), KeyModifiers::CONTROL, Command::Redo),
];

pub fn resolve(key: &KeyEvent) -> Option<Command> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }
    if let Some((_, _, command)) = TABLE
        .iter()
        .find(|(code, modifiers, _)| *code == key.code && *modifiers == key.modifiers)
    {
        return Some(*command);
    }
    match key.code {
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(Command::Insert(character))
        }
        _ => None,
    }
}

pub fn resolve_mouse(mouse: &MouseEvent) -> Option<Command> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => Some(Command::Click {
            column: mouse.column,
            row: mouse.row,
        }),
        MouseEventKind::Drag(MouseButton::Left) => Some(Command::Drag {
            column: mouse.column,
            row: mouse.row,
        }),
        MouseEventKind::ScrollUp => Some(Command::ScrollLines(-3)),
        MouseEventKind::ScrollDown => Some(Command::ScrollLines(3)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{
        KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
        MouseEventKind,
    };

    use super::{resolve, resolve_mouse};
    use crate::command::Command;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn released(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn movement_keys_resolve() {
        assert_eq!(
            resolve(&key(KeyCode::Left, KeyModifiers::NONE)),
            Some(Command::MoveLeft { select: false })
        );
        assert_eq!(
            resolve(&key(KeyCode::Right, KeyModifiers::NONE)),
            Some(Command::MoveRight { select: false })
        );
        assert_eq!(
            resolve(&key(KeyCode::Up, KeyModifiers::NONE)),
            Some(Command::MoveUp { select: false })
        );
        assert_eq!(
            resolve(&key(KeyCode::Down, KeyModifiers::NONE)),
            Some(Command::MoveDown { select: false })
        );
        assert_eq!(
            resolve(&key(KeyCode::Home, KeyModifiers::NONE)),
            Some(Command::Home { select: false })
        );
        assert_eq!(
            resolve(&key(KeyCode::End, KeyModifiers::NONE)),
            Some(Command::End { select: false })
        );
        assert_eq!(
            resolve(&key(KeyCode::PageUp, KeyModifiers::NONE)),
            Some(Command::PageUp { select: false })
        );
        assert_eq!(
            resolve(&key(KeyCode::PageDown, KeyModifiers::NONE)),
            Some(Command::PageDown { select: false })
        );
    }

    #[test]
    fn shift_movement_selects() {
        assert_eq!(
            resolve(&key(KeyCode::Left, KeyModifiers::SHIFT)),
            Some(Command::MoveLeft { select: true })
        );
        assert_eq!(
            resolve(&key(KeyCode::Right, KeyModifiers::SHIFT)),
            Some(Command::MoveRight { select: true })
        );
        assert_eq!(
            resolve(&key(KeyCode::Up, KeyModifiers::SHIFT)),
            Some(Command::MoveUp { select: true })
        );
        assert_eq!(
            resolve(&key(KeyCode::Down, KeyModifiers::SHIFT)),
            Some(Command::MoveDown { select: true })
        );
        assert_eq!(
            resolve(&key(KeyCode::Home, KeyModifiers::SHIFT)),
            Some(Command::Home { select: true })
        );
        assert_eq!(
            resolve(&key(KeyCode::End, KeyModifiers::SHIFT)),
            Some(Command::End { select: true })
        );
        assert_eq!(
            resolve(&key(KeyCode::PageUp, KeyModifiers::SHIFT)),
            Some(Command::PageUp { select: true })
        );
        assert_eq!(
            resolve(&key(KeyCode::PageDown, KeyModifiers::SHIFT)),
            Some(Command::PageDown { select: true })
        );
    }

    #[test]
    fn alt_arrows_move_lines() {
        assert_eq!(
            resolve(&key(KeyCode::Up, KeyModifiers::ALT)),
            Some(Command::MoveLineUp)
        );
        assert_eq!(
            resolve(&key(KeyCode::Down, KeyModifiers::ALT)),
            Some(Command::MoveLineDown)
        );
    }

    #[test]
    fn typing_inserts_characters() {
        assert_eq!(
            resolve(&key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(Command::Insert('a'))
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('A'), KeyModifiers::SHIFT)),
            Some(Command::Insert('A'))
        );
        assert_eq!(
            resolve(&key(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Command::Insert('\n'))
        );
        assert_eq!(
            resolve(&key(KeyCode::Tab, KeyModifiers::NONE)),
            Some(Command::Insert('\t'))
        );
    }

    #[test]
    fn control_letters_resolve_to_commands() {
        assert_eq!(
            resolve(&key(KeyCode::Char('s'), KeyModifiers::CONTROL)),
            Some(Command::Save)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            Some(Command::Quit)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('a'), KeyModifiers::CONTROL)),
            Some(Command::SelectAll)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('x'), KeyModifiers::CONTROL)),
            Some(Command::Cut)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Command::Copy)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('v'), KeyModifiers::CONTROL)),
            Some(Command::Paste)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('z'), KeyModifiers::CONTROL)),
            Some(Command::Undo)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('y'), KeyModifiers::CONTROL)),
            Some(Command::Redo)
        );
    }

    #[test]
    fn terminal_control_codes_keep_their_meaning() {
        assert_eq!(
            resolve(&key(KeyCode::Char('h'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('i'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('j'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('m'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            resolve(&key(KeyCode::Backspace, KeyModifiers::NONE)),
            Some(Command::Backspace)
        );
        assert_eq!(
            resolve(&key(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Command::Insert('\n'))
        );
    }

    #[test]
    fn release_events_are_ignored() {
        assert_eq!(
            resolve(&released(KeyCode::Char('a'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            resolve(&released(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            None
        );
    }

    #[test]
    fn control_shift_letter_is_not_bound() {
        assert_eq!(
            resolve(&key(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )),
            None
        );
    }

    #[test]
    fn synthetic_key_sequence_edits_editor() {
        use crate::buffer::Buffer;
        use crate::editor::Editor;

        let mut editor = Editor::new(Buffer::from_text(""));
        editor.resize(20, 5);
        for character in ['h', 'i'] {
            let command = resolve(&key(KeyCode::Char(character), KeyModifiers::NONE));
            assert!(command.is_some_and(|command| editor.apply(command)));
        }
        assert_eq!(editor.buffer().text(), "hi");
        let command = resolve(&key(KeyCode::Left, KeyModifiers::NONE));
        assert!(command.is_some_and(|command| editor.apply(command)));
        let command = resolve(&key(KeyCode::Char('X'), KeyModifiers::SHIFT));
        assert!(command.is_some_and(|command| editor.apply(command)));
        assert_eq!(editor.buffer().text(), "hXi");
    }

    #[test]
    fn mouse_maps_to_commands() {
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 12,
            row: 4,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(
            resolve_mouse(&click),
            Some(Command::Click { column: 12, row: 4 })
        );
        let drag = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 7,
            row: 3,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(
            resolve_mouse(&drag),
            Some(Command::Drag { column: 7, row: 3 })
        );
        let wheel_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(resolve_mouse(&wheel_up), Some(Command::ScrollLines(-3)));
        let wheel_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(resolve_mouse(&wheel_down), Some(Command::ScrollLines(3)));
    }

    #[test]
    fn mouse_release_is_ignored() {
        let release = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(resolve_mouse(&release), None);
    }
}
