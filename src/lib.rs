pub mod buffer;
pub mod clipboard;
pub mod command;
pub mod editor;
pub mod event;
pub mod keymap;
pub mod search;
pub mod ui;
pub mod view;

#[cfg(test)]
mod test_support;

pub use buffer::{Buffer, BufferError, BufferId, CharIdx, LineCol, LineEnding, Transaction};
pub use command::Command;
pub use editor::{Editor, Viewport};
pub use event::Event;
