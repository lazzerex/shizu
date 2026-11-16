#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    MoveLeft { select: bool },
    MoveRight { select: bool },
    MoveUp { select: bool },
    MoveDown { select: bool },
    Home { select: bool },
    End { select: bool },
    PageUp { select: bool },
    PageDown { select: bool },
    Insert(char),
    Backspace,
    Delete,
    SelectAll,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    MoveLineUp,
    MoveLineDown,
    Save,
    Quit,
    Cancel,
    OpenFind,
    OpenReplace,
    OpenGoToLine,
    FindNext,
    FindPrevious,
    PromptInsert(char),
    PromptBackspace,
    PromptNext,
    PromptPrevious,
    PromptToggleField,
    ReplaceAll,
    Click { column: u16, row: u16 },
    Drag { column: u16, row: u16 },
    ScrollLines(isize),
    ToggleSidebar,
    CloseTab,
    NextTab,
    PreviousTab,
    TreeUp,
    TreeDown,
    TreeLeft,
    TreeRight,
    TreeOpen,
    OpenPalette,
    PaletteUp,
    PaletteDown,
    PaletteInsert(char),
    PaletteBackspace,
    PaletteAccept,
}

impl Command {
    pub const fn description(self) -> Option<&'static str> {
        match self {
            Self::MoveLeft { select: false } => Some("Move left"),
            Self::MoveLeft { select: true } => Some("Select left"),
            Self::MoveRight { select: false } => Some("Move right"),
            Self::MoveRight { select: true } => Some("Select right"),
            Self::MoveUp { select: false } => Some("Move up"),
            Self::MoveUp { select: true } => Some("Select up"),
            Self::MoveDown { select: false } => Some("Move down"),
            Self::MoveDown { select: true } => Some("Select down"),
            Self::Home { select: false } => Some("Start of line"),
            Self::Home { select: true } => Some("Select to start"),
            Self::End { select: false } => Some("End of line"),
            Self::End { select: true } => Some("Select to end"),
            Self::PageUp { select: false } => Some("Page up"),
            Self::PageUp { select: true } => Some("Select page up"),
            Self::PageDown { select: false } => Some("Page down"),
            Self::PageDown { select: true } => Some("Select page down"),
            Self::Backspace => Some("Backspace"),
            Self::Delete => Some("Delete"),
            Self::SelectAll => Some("Select all"),
            Self::Cut => Some("Cut"),
            Self::Copy => Some("Copy"),
            Self::Paste => Some("Paste"),
            Self::Undo => Some("Undo"),
            Self::Redo => Some("Redo"),
            Self::MoveLineUp => Some("Move line up"),
            Self::MoveLineDown => Some("Move line down"),
            Self::Save => Some("Save"),
            Self::Quit => Some("Quit"),
            Self::Cancel => Some("Cancel"),
            Self::OpenFind => Some("Find"),
            Self::OpenReplace => Some("Replace"),
            Self::OpenGoToLine => Some("Go to line"),
            Self::FindNext => Some("Next match"),
            Self::FindPrevious => Some("Previous match"),
            Self::ToggleSidebar => Some("Toggle sidebar"),
            Self::CloseTab => Some("Close tab"),
            Self::NextTab => Some("Next tab"),
            Self::PreviousTab => Some("Previous tab"),
            Self::Insert(_)
            | Self::PromptInsert(_)
            | Self::PromptBackspace
            | Self::PromptNext
            | Self::PromptPrevious
            | Self::PromptToggleField
            | Self::ReplaceAll
            | Self::Click { .. }
            | Self::Drag { .. }
            | Self::ScrollLines(_)
            | Self::TreeUp
            | Self::TreeDown
            | Self::TreeLeft
            | Self::TreeRight
            | Self::TreeOpen
            | Self::OpenPalette
            | Self::PaletteUp
            | Self::PaletteDown
            | Self::PaletteInsert(_)
            | Self::PaletteBackspace
            | Self::PaletteAccept => None,
        }
    }
}
