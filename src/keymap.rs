use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::command::Command;
use crate::editor::Focus;

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
    (KeyCode::Char('f'), KeyModifiers::CONTROL, Command::OpenFind),
    (
        KeyCode::Char('r'),
        KeyModifiers::CONTROL,
        Command::OpenReplace,
    ),
    (
        KeyCode::Char('g'),
        KeyModifiers::CONTROL,
        Command::OpenGoToLine,
    ),
    (KeyCode::F(3), KeyModifiers::NONE, Command::FindNext),
    (KeyCode::F(3), KeyModifiers::SHIFT, Command::FindPrevious),
];

pub fn resolve(key: &KeyEvent, focus: Focus) -> Option<Command> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }
    match focus {
        Focus::Editor => resolve_editor(key),
        Focus::Find => resolve_find(key),
        Focus::Replace => resolve_replace(key),
        Focus::GoToLine => resolve_goto(key),
    }
}

fn resolve_editor(key: &KeyEvent) -> Option<Command> {
    if let Some((_, _, command)) = TABLE
        .iter()
        .find(|(code, modifiers, _)| *code == key.code && *modifiers == key.modifiers)
    {
        return Some(*command);
    }
    prompt_character(key).map(Command::Insert)
}

fn resolve_find(key: &KeyEvent) -> Option<Command> {
    match (key.code, key.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => Some(Command::Cancel),
        (KeyCode::Enter, KeyModifiers::NONE) => Some(Command::PromptNext),
        (KeyCode::Backspace, KeyModifiers::NONE) => Some(Command::PromptBackspace),
        (KeyCode::Char('p'), KeyModifiers::CONTROL) => Some(Command::PromptPrevious),
        (KeyCode::F(3), KeyModifiers::NONE) => Some(Command::FindNext),
        (KeyCode::F(3), KeyModifiers::SHIFT) => Some(Command::FindPrevious),
        _ => prompt_character(key).map(Command::PromptInsert),
    }
}

fn resolve_replace(key: &KeyEvent) -> Option<Command> {
    match (key.code, key.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => Some(Command::Cancel),
        (KeyCode::Enter, KeyModifiers::NONE) => Some(Command::PromptNext),
        (KeyCode::Tab, KeyModifiers::NONE) => Some(Command::PromptToggleField),
        (KeyCode::Backspace, KeyModifiers::NONE) => Some(Command::PromptBackspace),
        (KeyCode::Char('p'), KeyModifiers::CONTROL) => Some(Command::PromptPrevious),
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => Some(Command::ReplaceAll),
        _ => prompt_character(key).map(Command::PromptInsert),
    }
}

fn resolve_goto(key: &KeyEvent) -> Option<Command> {
    match (key.code, key.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => Some(Command::Cancel),
        (KeyCode::Enter, KeyModifiers::NONE) => Some(Command::PromptNext),
        (KeyCode::Backspace, KeyModifiers::NONE) => Some(Command::PromptBackspace),
        _ => prompt_character(key).map(Command::PromptInsert),
    }
}

fn prompt_character(key: &KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(character)
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
    use crate::editor::Focus;

    fn resolve_editor(key: &KeyEvent) -> Option<Command> {
        resolve(key, Focus::Editor)
    }

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
            resolve_editor(&key(KeyCode::Left, KeyModifiers::NONE)),
            Some(Command::MoveLeft { select: false })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Right, KeyModifiers::NONE)),
            Some(Command::MoveRight { select: false })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Up, KeyModifiers::NONE)),
            Some(Command::MoveUp { select: false })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Down, KeyModifiers::NONE)),
            Some(Command::MoveDown { select: false })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Home, KeyModifiers::NONE)),
            Some(Command::Home { select: false })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::End, KeyModifiers::NONE)),
            Some(Command::End { select: false })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::PageUp, KeyModifiers::NONE)),
            Some(Command::PageUp { select: false })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::PageDown, KeyModifiers::NONE)),
            Some(Command::PageDown { select: false })
        );
    }

    #[test]
    fn shift_movement_selects() {
        assert_eq!(
            resolve_editor(&key(KeyCode::Left, KeyModifiers::SHIFT)),
            Some(Command::MoveLeft { select: true })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Right, KeyModifiers::SHIFT)),
            Some(Command::MoveRight { select: true })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Up, KeyModifiers::SHIFT)),
            Some(Command::MoveUp { select: true })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Down, KeyModifiers::SHIFT)),
            Some(Command::MoveDown { select: true })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Home, KeyModifiers::SHIFT)),
            Some(Command::Home { select: true })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::End, KeyModifiers::SHIFT)),
            Some(Command::End { select: true })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::PageUp, KeyModifiers::SHIFT)),
            Some(Command::PageUp { select: true })
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::PageDown, KeyModifiers::SHIFT)),
            Some(Command::PageDown { select: true })
        );
    }

    #[test]
    fn alt_arrows_move_lines() {
        assert_eq!(
            resolve_editor(&key(KeyCode::Up, KeyModifiers::ALT)),
            Some(Command::MoveLineUp)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Down, KeyModifiers::ALT)),
            Some(Command::MoveLineDown)
        );
    }

    #[test]
    fn typing_inserts_characters() {
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(Command::Insert('a'))
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('A'), KeyModifiers::SHIFT)),
            Some(Command::Insert('A'))
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Command::Insert('\n'))
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Tab, KeyModifiers::NONE)),
            Some(Command::Insert('\t'))
        );
    }

    #[test]
    fn control_letters_resolve_to_commands() {
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('s'), KeyModifiers::CONTROL)),
            Some(Command::Save)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            Some(Command::Quit)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('a'), KeyModifiers::CONTROL)),
            Some(Command::SelectAll)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('x'), KeyModifiers::CONTROL)),
            Some(Command::Cut)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Command::Copy)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('v'), KeyModifiers::CONTROL)),
            Some(Command::Paste)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('z'), KeyModifiers::CONTROL)),
            Some(Command::Undo)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('y'), KeyModifiers::CONTROL)),
            Some(Command::Redo)
        );
    }

    #[test]
    fn terminal_control_codes_keep_their_meaning() {
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('h'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('i'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('j'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('m'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Backspace, KeyModifiers::NONE)),
            Some(Command::Backspace)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Command::Insert('\n'))
        );
    }

    #[test]
    fn release_events_are_ignored() {
        assert_eq!(
            resolve_editor(&released(KeyCode::Char('a'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            resolve_editor(&released(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            None
        );
    }

    #[test]
    fn control_shift_letter_is_not_bound() {
        assert_eq!(
            resolve_editor(&key(
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
            let command = resolve_editor(&key(KeyCode::Char(character), KeyModifiers::NONE));
            assert!(command.is_some_and(|command| editor.apply(command)));
        }
        assert_eq!(editor.buffer().text(), "hi");
        let command = resolve_editor(&key(KeyCode::Left, KeyModifiers::NONE));
        assert!(command.is_some_and(|command| editor.apply(command)));
        let command = resolve_editor(&key(KeyCode::Char('X'), KeyModifiers::SHIFT));
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

    #[test]
    fn editor_focus_binds_search_keys() {
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('f'), KeyModifiers::CONTROL)),
            Some(Command::OpenFind)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Some(Command::OpenReplace)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('g'), KeyModifiers::CONTROL)),
            Some(Command::OpenGoToLine)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::F(3), KeyModifiers::NONE)),
            Some(Command::FindNext)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::F(3), KeyModifiers::SHIFT)),
            Some(Command::FindPrevious)
        );
    }

    #[test]
    fn find_focus_routes_typing_and_navigation() {
        assert_eq!(
            resolve(&key(KeyCode::Char('a'), KeyModifiers::NONE), Focus::Find),
            Some(Command::PromptInsert('a'))
        );
        assert_eq!(
            resolve(&key(KeyCode::Enter, KeyModifiers::NONE), Focus::Find),
            Some(Command::PromptNext)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('p'), KeyModifiers::CONTROL), Focus::Find),
            Some(Command::PromptPrevious)
        );
        assert_eq!(
            resolve(&key(KeyCode::Backspace, KeyModifiers::NONE), Focus::Find),
            Some(Command::PromptBackspace)
        );
        assert_eq!(
            resolve(&key(KeyCode::Esc, KeyModifiers::NONE), Focus::Find),
            Some(Command::Cancel)
        );
    }

    #[test]
    fn replace_focus_binds_field_toggle_and_replace_all() {
        assert_eq!(
            resolve(&key(KeyCode::Tab, KeyModifiers::NONE), Focus::Replace),
            Some(Command::PromptToggleField)
        );
        assert_eq!(
            resolve(
                &key(KeyCode::Char('a'), KeyModifiers::CONTROL),
                Focus::Replace
            ),
            Some(Command::ReplaceAll)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('a'), KeyModifiers::NONE), Focus::Replace),
            Some(Command::PromptInsert('a'))
        );
    }

    #[test]
    fn goto_focus_routes_digits_and_enter() {
        assert_eq!(
            resolve(
                &key(KeyCode::Char('4'), KeyModifiers::NONE),
                Focus::GoToLine
            ),
            Some(Command::PromptInsert('4'))
        );
        assert_eq!(
            resolve(&key(KeyCode::Enter, KeyModifiers::NONE), Focus::GoToLine),
            Some(Command::PromptNext)
        );
        assert_eq!(
            resolve(&key(KeyCode::Esc, KeyModifiers::NONE), Focus::GoToLine),
            Some(Command::Cancel)
        );
    }

    #[test]
    fn prompt_focus_ignores_release_events() {
        assert_eq!(
            resolve(
                &released(KeyCode::Char('a'), KeyModifiers::NONE),
                Focus::Find
            ),
            None
        );
    }
}
