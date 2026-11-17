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
    (
        KeyCode::Char('p'),
        KeyModifiers::CONTROL,
        Command::OpenPalette,
    ),
    (KeyCode::F(3), KeyModifiers::NONE, Command::FindNext),
    (KeyCode::F(3), KeyModifiers::SHIFT, Command::FindPrevious),
    (
        KeyCode::Char('b'),
        KeyModifiers::CONTROL,
        Command::ToggleSidebar,
    ),
    (KeyCode::Char('w'), KeyModifiers::CONTROL, Command::CloseTab),
    (
        KeyCode::Char('t'),
        KeyModifiers::CONTROL,
        Command::ToggleTerminal,
    ),
    (KeyCode::PageDown, KeyModifiers::CONTROL, Command::NextTab),
    (KeyCode::PageUp, KeyModifiers::CONTROL, Command::PreviousTab),
];

const TREE_TABLE: &[(KeyCode, KeyModifiers, Command)] = &[
    (KeyCode::Up, KeyModifiers::NONE, Command::TreeUp),
    (KeyCode::Down, KeyModifiers::NONE, Command::TreeDown),
    (KeyCode::Left, KeyModifiers::NONE, Command::TreeLeft),
    (KeyCode::Right, KeyModifiers::NONE, Command::TreeRight),
    (KeyCode::Enter, KeyModifiers::NONE, Command::TreeOpen),
    (KeyCode::Esc, KeyModifiers::NONE, Command::Cancel),
    (
        KeyCode::Char('p'),
        KeyModifiers::CONTROL,
        Command::OpenPalette,
    ),
    (
        KeyCode::Char('b'),
        KeyModifiers::CONTROL,
        Command::ToggleSidebar,
    ),
    (KeyCode::Char('w'), KeyModifiers::CONTROL, Command::CloseTab),
    (KeyCode::Char('s'), KeyModifiers::CONTROL, Command::Save),
    (KeyCode::Char('q'), KeyModifiers::CONTROL, Command::Quit),
    (
        KeyCode::Char('t'),
        KeyModifiers::CONTROL,
        Command::ToggleTerminal,
    ),
    (KeyCode::PageDown, KeyModifiers::CONTROL, Command::NextTab),
    (KeyCode::PageUp, KeyModifiers::CONTROL, Command::PreviousTab),
];

const TERMINAL_TABLE: &[(KeyCode, KeyModifiers, Command)] = &[(
    KeyCode::Char('t'),
    KeyModifiers::CONTROL,
    Command::ToggleTerminal,
)];

const FIND_TABLE: &[(KeyCode, KeyModifiers, Command)] = &[
    (KeyCode::Esc, KeyModifiers::NONE, Command::Cancel),
    (KeyCode::Enter, KeyModifiers::NONE, Command::PromptNext),
    (
        KeyCode::Backspace,
        KeyModifiers::NONE,
        Command::PromptBackspace,
    ),
    (
        KeyCode::Char('p'),
        KeyModifiers::CONTROL,
        Command::PromptPrevious,
    ),
    (KeyCode::F(3), KeyModifiers::NONE, Command::FindNext),
    (KeyCode::F(3), KeyModifiers::SHIFT, Command::FindPrevious),
];

const REPLACE_TABLE: &[(KeyCode, KeyModifiers, Command)] = &[
    (KeyCode::Esc, KeyModifiers::NONE, Command::Cancel),
    (KeyCode::Enter, KeyModifiers::NONE, Command::PromptNext),
    (KeyCode::Tab, KeyModifiers::NONE, Command::PromptToggleField),
    (
        KeyCode::Backspace,
        KeyModifiers::NONE,
        Command::PromptBackspace,
    ),
    (
        KeyCode::Char('p'),
        KeyModifiers::CONTROL,
        Command::PromptPrevious,
    ),
    (
        KeyCode::Char('a'),
        KeyModifiers::CONTROL,
        Command::ReplaceAll,
    ),
];

const GOTO_TABLE: &[(KeyCode, KeyModifiers, Command)] = &[
    (KeyCode::Esc, KeyModifiers::NONE, Command::Cancel),
    (KeyCode::Enter, KeyModifiers::NONE, Command::PromptNext),
    (
        KeyCode::Backspace,
        KeyModifiers::NONE,
        Command::PromptBackspace,
    ),
];

const PALETTE_TABLE: &[(KeyCode, KeyModifiers, Command)] = &[
    (KeyCode::Up, KeyModifiers::NONE, Command::PaletteUp),
    (KeyCode::Down, KeyModifiers::NONE, Command::PaletteDown),
    (KeyCode::Enter, KeyModifiers::NONE, Command::PaletteAccept),
    (KeyCode::Esc, KeyModifiers::NONE, Command::Cancel),
];

#[derive(Debug)]
pub(crate) struct CommandEntry {
    pub(crate) command: Command,
    pub(crate) label: &'static str,
    pub(crate) keys: Option<String>,
}

struct HintSpec {
    command: Command,
    label: &'static str,
}

const EDITOR_HINTS: &[HintSpec] = &[
    HintSpec {
        command: Command::OpenPalette,
        label: "Palette",
    },
    HintSpec {
        command: Command::Save,
        label: "Save",
    },
    HintSpec {
        command: Command::OpenFind,
        label: "Find",
    },
    HintSpec {
        command: Command::ToggleSidebar,
        label: "Sidebar",
    },
    HintSpec {
        command: Command::ToggleTerminal,
        label: "Terminal",
    },
    HintSpec {
        command: Command::Quit,
        label: "Quit",
    },
];

const TREE_HINTS: &[HintSpec] = &[
    HintSpec {
        command: Command::OpenPalette,
        label: "Palette",
    },
    HintSpec {
        command: Command::TreeOpen,
        label: "Open",
    },
    HintSpec {
        command: Command::TreeRight,
        label: "Expand",
    },
    HintSpec {
        command: Command::TreeLeft,
        label: "Collapse",
    },
    HintSpec {
        command: Command::ToggleSidebar,
        label: "Close",
    },
    HintSpec {
        command: Command::Quit,
        label: "Quit",
    },
];

const FIND_HINTS: &[HintSpec] = &[
    HintSpec {
        command: Command::FindNext,
        label: "Next",
    },
    HintSpec {
        command: Command::FindPrevious,
        label: "Previous",
    },
    HintSpec {
        command: Command::Cancel,
        label: "Close",
    },
];

const REPLACE_HINTS: &[HintSpec] = &[
    HintSpec {
        command: Command::PromptNext,
        label: "Next",
    },
    HintSpec {
        command: Command::PromptToggleField,
        label: "Field",
    },
    HintSpec {
        command: Command::ReplaceAll,
        label: "All",
    },
    HintSpec {
        command: Command::Cancel,
        label: "Close",
    },
];

const GOTO_HINTS: &[HintSpec] = &[
    HintSpec {
        command: Command::PromptNext,
        label: "Go",
    },
    HintSpec {
        command: Command::Cancel,
        label: "Close",
    },
];

const TERMINAL_HINTS: &[HintSpec] = &[HintSpec {
    command: Command::ToggleTerminal,
    label: "Close",
}];

const PALETTE_HINTS: &[HintSpec] = &[
    HintSpec {
        command: Command::PaletteAccept,
        label: "Open",
    },
    HintSpec {
        command: Command::PaletteUp,
        label: "Up",
    },
    HintSpec {
        command: Command::PaletteDown,
        label: "Down",
    },
    HintSpec {
        command: Command::Cancel,
        label: "Close",
    },
];

pub fn resolve(key: &KeyEvent, focus: Focus) -> Option<Command> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }
    match focus {
        Focus::Editor => resolve_editor(key),
        Focus::Tree => resolve_table(key, TREE_TABLE),
        Focus::Find => resolve_find(key),
        Focus::Replace => resolve_replace(key),
        Focus::GoToLine => resolve_goto(key),
        Focus::Palette => resolve_palette(key),
        Focus::Terminal => resolve_terminal(key),
    }
}

fn resolve_terminal(key: &KeyEvent) -> Option<Command> {
    resolve_table(key, TERMINAL_TABLE).or(Some(Command::TerminalKey(key.code, key.modifiers)))
}

fn resolve_editor(key: &KeyEvent) -> Option<Command> {
    resolve_table(key, TABLE).or_else(|| prompt_character(key).map(Command::Insert))
}

fn resolve_find(key: &KeyEvent) -> Option<Command> {
    resolve_table(key, FIND_TABLE).or_else(|| prompt_character(key).map(Command::PromptInsert))
}

fn resolve_replace(key: &KeyEvent) -> Option<Command> {
    resolve_table(key, REPLACE_TABLE).or_else(|| prompt_character(key).map(Command::PromptInsert))
}

fn resolve_goto(key: &KeyEvent) -> Option<Command> {
    resolve_table(key, GOTO_TABLE).or_else(|| prompt_character(key).map(Command::PromptInsert))
}

fn resolve_palette(key: &KeyEvent) -> Option<Command> {
    if let Some(command) = resolve_table(key, PALETTE_TABLE) {
        return Some(command);
    }
    match key.code {
        KeyCode::Backspace => Some(Command::PaletteBackspace),
        _ => prompt_character(key).map(Command::PaletteInsert),
    }
}

fn resolve_table(key: &KeyEvent, table: &[(KeyCode, KeyModifiers, Command)]) -> Option<Command> {
    table
        .iter()
        .find(|(code, modifiers, _)| *code == key.code && *modifiers == key.modifiers)
        .map(|(_, _, command)| *command)
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

pub(crate) fn command_entries() -> Vec<CommandEntry> {
    let mut entries: Vec<CommandEntry> = Vec::new();
    for (code, modifiers, command) in TABLE {
        let Some(label) = command.description() else {
            continue;
        };
        if entries.iter().any(|entry| entry.command == *command) {
            continue;
        }
        entries.push(CommandEntry {
            command: *command,
            label,
            keys: Some(key_label(*code, *modifiers)),
        });
    }
    entries
}

pub fn hint_text(focus: Focus) -> String {
    let table = table(focus);
    hint_specs(focus)
        .iter()
        .filter_map(|spec| {
            let (code, modifiers, _) = table
                .iter()
                .find(|(_, _, command)| *command == spec.command)?;
            Some(format!("{} {}", key_label(*code, *modifiers), spec.label))
        })
        .collect::<Vec<_>>()
        .join("  ")
}

fn table(focus: Focus) -> &'static [(KeyCode, KeyModifiers, Command)] {
    match focus {
        Focus::Editor => TABLE,
        Focus::Tree => TREE_TABLE,
        Focus::Find => FIND_TABLE,
        Focus::Replace => REPLACE_TABLE,
        Focus::GoToLine => GOTO_TABLE,
        Focus::Palette => PALETTE_TABLE,
        Focus::Terminal => TERMINAL_TABLE,
    }
}

fn hint_specs(focus: Focus) -> &'static [HintSpec] {
    match focus {
        Focus::Editor => EDITOR_HINTS,
        Focus::Tree => TREE_HINTS,
        Focus::Find => FIND_HINTS,
        Focus::Replace => REPLACE_HINTS,
        Focus::GoToLine => GOTO_HINTS,
        Focus::Palette => PALETTE_HINTS,
        Focus::Terminal => TERMINAL_HINTS,
    }
}

fn key_label(code: KeyCode, modifiers: KeyModifiers) -> String {
    let mut label = String::new();
    if modifiers.contains(KeyModifiers::CONTROL) {
        label.push_str("Ctrl+");
    }
    if modifiers.contains(KeyModifiers::ALT) {
        label.push_str("Alt+");
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        label.push_str("Shift+");
    }
    let name = match code {
        KeyCode::Char(character) => {
            if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
                character.to_ascii_uppercase().to_string()
            } else {
                character.to_string()
            }
        }
        KeyCode::Enter => "Enter".to_owned(),
        KeyCode::Esc => "Esc".to_owned(),
        KeyCode::Tab => "Tab".to_owned(),
        KeyCode::Backspace => "Backspace".to_owned(),
        KeyCode::Delete => "Delete".to_owned(),
        KeyCode::Up => "Up".to_owned(),
        KeyCode::Down => "Down".to_owned(),
        KeyCode::Left => "Left".to_owned(),
        KeyCode::Right => "Right".to_owned(),
        KeyCode::Home => "Home".to_owned(),
        KeyCode::End => "End".to_owned(),
        KeyCode::PageUp => "PageUp".to_owned(),
        KeyCode::PageDown => "PageDown".to_owned(),
        KeyCode::F(number) => format!("F{number}"),
        _ => "Key".to_owned(),
    };
    label.push_str(&name);
    label
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

    #[test]
    fn editor_focus_binds_sidebar_and_tabs() {
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('b'), KeyModifiers::CONTROL)),
            Some(Command::ToggleSidebar)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('w'), KeyModifiers::CONTROL)),
            Some(Command::CloseTab)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::PageDown, KeyModifiers::CONTROL)),
            Some(Command::NextTab)
        );
        assert_eq!(
            resolve_editor(&key(KeyCode::PageUp, KeyModifiers::CONTROL)),
            Some(Command::PreviousTab)
        );
    }

    #[test]
    fn editor_and_tree_focus_open_the_terminal() {
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('t'), KeyModifiers::CONTROL)),
            Some(Command::ToggleTerminal)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('t'), KeyModifiers::CONTROL), Focus::Tree),
            Some(Command::ToggleTerminal)
        );
    }

    #[test]
    fn editor_focus_opens_the_palette() {
        assert_eq!(
            resolve_editor(&key(KeyCode::Char('p'), KeyModifiers::CONTROL)),
            Some(Command::OpenPalette)
        );
    }

    #[test]
    fn palette_focus_routes_filtering_and_navigation() {
        assert_eq!(
            resolve(&key(KeyCode::Char('m'), KeyModifiers::NONE), Focus::Palette),
            Some(Command::PaletteInsert('m'))
        );
        assert_eq!(
            resolve(&key(KeyCode::Backspace, KeyModifiers::NONE), Focus::Palette),
            Some(Command::PaletteBackspace)
        );
        assert_eq!(
            resolve(&key(KeyCode::Up, KeyModifiers::NONE), Focus::Palette),
            Some(Command::PaletteUp)
        );
        assert_eq!(
            resolve(&key(KeyCode::Down, KeyModifiers::NONE), Focus::Palette),
            Some(Command::PaletteDown)
        );
        assert_eq!(
            resolve(&key(KeyCode::Enter, KeyModifiers::NONE), Focus::Palette),
            Some(Command::PaletteAccept)
        );
        assert_eq!(
            resolve(&key(KeyCode::Esc, KeyModifiers::NONE), Focus::Palette),
            Some(Command::Cancel)
        );
    }

    #[test]
    fn palette_focus_ignores_control_letters() {
        assert_eq!(
            resolve(
                &key(KeyCode::Char('p'), KeyModifiers::CONTROL),
                Focus::Palette
            ),
            None
        );
    }

    #[test]
    fn command_entries_come_from_the_editor_table_without_duplicates() {
        let entries = super::command_entries();
        let save = entries
            .iter()
            .find(|entry| entry.command == Command::Save)
            .expect("save entry");
        assert_eq!(save.label, "Save");
        assert_eq!(save.keys.as_deref(), Some("Ctrl+S"));

        let moves = entries
            .iter()
            .filter(|entry| entry.command == Command::MoveLeft { select: false })
            .count();
        assert_eq!(moves, 1);
        assert!(
            entries
                .iter()
                .all(|entry| entry.command.description().is_some())
        );
    }

    #[test]
    fn hint_text_uses_the_current_focus_table() {
        let editor_hints = super::hint_text(Focus::Editor);
        assert!(editor_hints.contains("Ctrl+P Palette"));
        assert!(editor_hints.contains("Ctrl+S Save"));

        let palette_hints = super::hint_text(Focus::Palette);
        assert!(palette_hints.contains("Enter Open"));
        assert!(palette_hints.contains("Esc Close"));
        assert!(!palette_hints.contains("Ctrl+S"));
    }

    #[test]
    fn terminal_focus_forwards_keys_and_keeps_the_close_binding() {
        assert_eq!(
            resolve(
                &key(KeyCode::Char('x'), KeyModifiers::NONE),
                Focus::Terminal
            ),
            Some(Command::TerminalKey(KeyCode::Char('x'), KeyModifiers::NONE))
        );
        assert_eq!(
            resolve(
                &key(KeyCode::Char('t'), KeyModifiers::CONTROL),
                Focus::Terminal
            ),
            Some(Command::ToggleTerminal)
        );
    }

    #[test]
    fn hint_text_mentions_the_terminal_binding() {
        assert!(super::hint_text(Focus::Editor).contains("Ctrl+T Terminal"));
        assert!(super::hint_text(Focus::Terminal).contains("Ctrl+T Close"));
    }

    #[test]
    fn tree_focus_navigates_and_can_leave() {
        assert_eq!(
            resolve(&key(KeyCode::Up, KeyModifiers::NONE), Focus::Tree),
            Some(Command::TreeUp)
        );
        assert_eq!(
            resolve(&key(KeyCode::Down, KeyModifiers::NONE), Focus::Tree),
            Some(Command::TreeDown)
        );
        assert_eq!(
            resolve(&key(KeyCode::Left, KeyModifiers::NONE), Focus::Tree),
            Some(Command::TreeLeft)
        );
        assert_eq!(
            resolve(&key(KeyCode::Right, KeyModifiers::NONE), Focus::Tree),
            Some(Command::TreeRight)
        );
        assert_eq!(
            resolve(&key(KeyCode::Enter, KeyModifiers::NONE), Focus::Tree),
            Some(Command::TreeOpen)
        );
        assert_eq!(
            resolve(&key(KeyCode::Esc, KeyModifiers::NONE), Focus::Tree),
            Some(Command::Cancel)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('b'), KeyModifiers::CONTROL), Focus::Tree),
            Some(Command::ToggleSidebar)
        );
        assert_eq!(
            resolve(&key(KeyCode::Char('p'), KeyModifiers::CONTROL), Focus::Tree),
            Some(Command::OpenPalette)
        );
    }
}
