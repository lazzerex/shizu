# shizu

shizu is a text editor that runs in a terminal. It stays modeless like nano and micro, but adds tree-sitter syntax highlighting, themes, mouse selection, and search and replace; unlike Helix, there are no modes to learn. I built it because [TODO: why]. It's for people who want nano's modeless editing with syntax highlighting they don't have to configure.

![shizu editing a Rust file](docs/screenshot.png)

*Screenshot placeholder.*

Status: pre-1.0. Editing, saving, search, undo, and syntax highlighting work on Linux; the Windows and macOS code paths are written but untested.

## Install

shizu isn't published on crates.io yet. Install from a checkout:

```sh
git clone <repository-url>
cd shizu
cargo install --path .
```

Or build a release binary without installing it:

```sh
cargo build --release
./target/release/shizu path/to/file
```

Starting without a path opens an empty buffer. It runs without a config file; configuration is optional.

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

## Design notes

The buffer is rope-backed, and the cursor moves by grapheme cluster rather than byte or `char`. Layout uses display width, so wide characters and tabs line up.

Saves write a temporary file in the same directory, sync it, and rename it over the target. The original is never truncated in place.

Syntax highlighting parses a 128-line block around the viewport plus 64 lines of context, and caches spans by buffer revision and window. Cost stays bounded as files grow, at the price of context at the top of the window.

## Non-goals

shizu won't become a full IDE. There's no debugger, build runner, or project management, and no plugin scripting language. It won't do modal editing. It won't add telemetry, ads, or auto-updates.

## Known limits

A block comment or string longer than 64 lines can lose color at the top of the highlight window, because highlighting starts from a nearby line rather than the beginning of the file.

Embedded languages aren't re-highlighted. An expression inside a Python f-string or a command substitution in Bash uses the outer scope.

C++ isn't supported, and `.h` files are treated as C. Only the C grammar is installed.

A new buffer with no path can't be saved yet; save reports that there is no path.

Windows and macOS key handling, mouse behavior, and clipboard integration are untested on those systems.

There's no file tree, tabs, panes, command palette, embedded terminal, or language server support yet.
