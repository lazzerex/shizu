# shizu

shizu is a text editor that runs in a terminal. It keeps nano's modeless editing but uses tree-sitter for syntax highlighting, and unlike Helix you never switch modes to select, search, or save. I built it because I wanted the highlighting of a modal editor with the keybindings I already know, plus a theme that isn't an afterthought. It's for people who work over SSH and want a capable editor without a plugin setup.

Status: pre-1.0. Editing, saving, search, undo, and syntax highlighting work on Linux; the Windows and macOS code paths are written but untested.

## Why shizu

nano is the closest in feel. shizu keeps modeless editing, then adds tree-sitter parsing, its own themes, and selection and mouse handling that don't feel bolted on.

micro is modeless and extensible through plugins. shizu has no plugin system, and its entire config is two keys.

Helix has the tree-sitter highlighting and a better structural editing model, but it's modal. shizu is not.

The editor is one Rust executable. It runs without a config file, though you can add one for the theme and tab width.

## Install

shizu isn't on crates.io. Install straight from the repository:

```sh
cargo install --git https://github.com/lazzerex/shizu.git
```

Or build a release binary from a checkout:

```sh
git clone https://github.com/lazzerex/shizu.git
cd shizu
cargo build --release
./target/release/shizu path/to/file
```

Starting without a path opens an empty buffer.

## What it does

- Modeless editing with selection, cut, copy, paste, undo, redo, and line moves.
- Syntax highlighting for Rust, JSON, TOML, Python, JavaScript, JSX, TypeScript, TSX, Go, C, and Bash.
- Incremental find, replace, replace-all, and go to line.
- Two themes, `sakura` and `sakura-light`, with truecolor and 256-color palettes and a plain style for `NO_COLOR`.
- Mouse click, drag, and wheel.
- Optional config for theme and tab width.

It also preserves LF/CRLF and the final newline, saves through a temporary file and rename, detects changes on disk before saving, and refuses read-only files or invalid UTF-8.

## Keys

| Key | Action |
| --- | --- |
| Arrows, Home, End, PageUp, PageDown | Move the cursor |
| Shift plus a movement key | Extend the selection |
| Alt+Up, Alt+Down | Move the current line |
| Ctrl+A | Select all |
| Ctrl+X, Ctrl+C, Ctrl+V | Cut, copy, paste |
| Ctrl+Z, Ctrl+Y | Undo, redo |
| Ctrl+S | Save |
| Ctrl+Q | Quit, with confirmation when dirty |
| Ctrl+F | Find |
| Ctrl+R | Replace |
| Ctrl+G | Go to line |
| F3, Shift+F3 | Next match, previous match |
| Esc | Close the prompt or cancel |

In the find prompt, Enter goes to the next match and Ctrl+P goes to the previous one. In the replace prompt, Tab switches between the find and replace fields, and Ctrl+A replaces all matches. Mouse click, drag, and wheel work in the editor area.

## Configuration

The config file is optional. Missing files use the defaults.

- Linux and macOS: `$XDG_CONFIG_HOME/shizu/config.toml`, falling back to `~/.config/shizu/config.toml`
- Windows: `%APPDATA%\shizu\config.toml`

```toml
theme = "sakura"
tab_width = 4
```

`theme` accepts `sakura` or `sakura-light`. An unknown theme falls back to `sakura` and shows a message on the status bar. `tab_width` is clamped to at least 1. `NO_COLOR` selects plain styles, and `COLORTERM=truecolor` or `COLORTERM=24bit` selects truecolor; otherwise the 256-color palette is used.

## How it works

| Layer | Technology | Role |
| --- | --- | --- |
| Text | ropey | Rope-backed buffers and cheap line access |
| Terminal | ratatui + crossterm | Rendering, input, mouse, alternate screen |
| Highlighting | tree-sitter | Incremental parsing with per-language grammars |
| Clipboard | arboard | System clipboard integration |
| Config | serde + toml | Optional theme and tab-width file |

Input flows through one path: a crossterm event becomes an `Event`, `keymap::resolve` turns it into a `Command`, `Editor::apply` mutates the buffer and view, and `ui::draw` renders the result. The command enum is the seam between input and editing logic, which is why the same actions work from keys and the mouse.

The buffer is rope-backed, and the cursor moves by grapheme cluster rather than byte or `char`. Layout uses display width, so wide characters and tabs line up.

Saves write a temporary file in the same directory, sync it, and rename it over the target. The original is never truncated in place.

Syntax highlighting parses a 128-line block around the viewport plus 64 lines of context, and caches spans by buffer revision and window. Cost stays bounded as files grow, at the price of context at the top of the window.

### Source layout

```text
src/
  main.rs          terminal setup, event loop, panic-safe restore
  buffer.rs        rope, transactions, load and atomic save
  view.rs          cursor, selection, scroll, layout columns
  editor/          command dispatch, movement, editing, prompts
  keymap.rs        key and mouse tables, focus-aware resolution
  search.rs        pure literal matcher
  syntax/          language registry and highlight cache
  theme.rs         palettes, color depth, contrast, styles
  config.rs        optional TOML config
  ui.rs            frame rendering
```

## Testing

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

There are 163 unit and render tests covering buffer edits, save behavior, scrolling, keymap resolution, selection, undo, search, themes, and highlighting. The terminal itself is exercised with a pty harness on Linux. Interactive behavior on Windows and macOS, clipboard interop, and a crash during save still need manual checks.

## Non-goals

shizu won't become a full IDE. There's no debugger, build runner, or project management, and no plugin scripting language. It won't do modal editing. It won't add telemetry, ads, or auto-updates.

## Known limits

A block comment or string longer than 64 lines can lose color at the top of the highlight window, because highlighting starts from a nearby line rather than the beginning of the file.

Embedded languages aren't re-highlighted. An expression inside a Python f-string or a command substitution in Bash uses the outer scope.

C++ isn't supported, and `.h` files are treated as C. Only the C grammar is installed.

A new buffer with no path can't be saved yet; save reports that there is no path.

Windows and macOS key handling, mouse behavior, and clipboard integration are untested on those systems.

There's no file tree, tabs, panes, command palette, embedded terminal, or language server support yet.
