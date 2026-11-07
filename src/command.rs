#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Home,
    End,
    PageUp,
    PageDown,
    Insert(char),
    Backspace,
    Delete,
    Save,
    Quit,
    Cancel,
    Click { column: u16, row: u16 },
    ScrollLines(isize),
}
