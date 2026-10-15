# Terminal

A clean, native terminal emulator for Linux, built with [CreamUI](https://github.com/sammwyy/creamui).
The executable is `dterm`; the desktop name is **Terminal**.

- One clean title for a single session; compact, closable tabs appear when you open another.
- Settings and **+** stay together, opposite the compositor’s window controls.
- System theme, live CreamUI theme updates, and hybrid compositor decorations.
- UTF-8, ANSI/256/truecolor text, cursor addressing, alternate screen and 10,000 lines of scrollback.
- Text selection, clipboard copy/paste and bracketed paste.
- Configurable shell (`bash` by default), with `sh` and `bash` fallbacks, or an explicit custom command.
- Settings open in a separate `dterm --settings` process: General, Shortcuts and About.
  Package version, author, license and repository links are embedded from `Cargo.toml`.

## Build

Keep `dterm`, `creamui`, and `delib` as sibling checkouts. Use stable Rust and install
a C/C++ compiler, `pkg-config`, D-Bus, Wayland and libxkbcommon development packages.
Fontconfig supplies the system monospace font.

```sh
cargo build --release --locked
cargo run --locked
cargo run --locked -- --settings
cargo run --locked -- --working-directory ~/Projects
cargo run --locked -- -e /bin/sh -c 'printf "hello\n"; exec /bin/sh'
```

The current CreamUI checkout includes the native keyboard interception API required
for Tab and function keys to reach terminal applications.

## Shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl+Shift+T | New tab |
| Ctrl+Shift+N | New window |
| Ctrl+Shift+W | Close tab (closing the last tab closes the window) |
| Ctrl+Tab / Ctrl+Shift+Tab | Next / previous tab |
| Alt+1 … Alt+9 | Select tab |
| Ctrl+Shift+C / Ctrl+Shift+V | Copy selection / paste |
| Ctrl+Comma | Open settings |
| Shift+PageUp / PageDown, mouse wheel | Scroll history |

Applications using the alternate screen keep their own screen contents. Mouse
reporting, hyperlinks and image protocols are not implemented yet.

## Configuration

`delib` resolves `$XDG_CONFIG_HOME/dterm/dterm.toml` (normally
`~/.config/dterm/dterm.toml`), followed by `$XDG_CONFIG_DIRS/dterm/dterm.toml`
for system defaults. A missing file uses the built-in defaults; settings writes
the user file atomically when you click **Save changes**.

```toml
shell = "bash"

[shortcuts]
new_tab = "Ctrl+Shift+T"
new_window = "Ctrl+Shift+N"
close_tab = "Ctrl+Shift+W"
next_tab = "Ctrl+Tab"
previous_tab = "Ctrl+Shift+Tab"
tab_1 = "Alt+1"
copy = "Ctrl+Shift+C"
paste = "Ctrl+Shift+V"
settings = "Ctrl+Comma"
scroll_up = "Shift+PageUp"
scroll_down = "Shift+PageDown"
```

Omitted shortcuts retain their defaults. `tab_1` through `tab_9` select a tab
by its one-based index; an empty string disables a shortcut. The Shortcuts tab
records keys directly, detects conflicts, and supports Escape to cancel or
Backspace/Delete to clear a binding. Ctrl, Shift, Alt, Super, navigation keys,
F1–F12, and named punctuation such as `Comma` and `Plus` are supported.

Saved shortcuts apply to open windows automatically. Shell changes apply to
new sessions: the configured executable is searched in `PATH`, then `sh`, then
`bash`; an explicit `-e` command is never replaced with a fallback shell.
Malformed configuration leaves the running window's previous bindings in place
and displays the error.

## Install

```sh
install -Dm755 target/release/dterm ~/.local/bin/dterm
install -Dm644 data/dterm.desktop ~/.local/share/applications/dterm.desktop
```

## Development

```sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```

Licensed under [Apache-2.0](LICENSE).
