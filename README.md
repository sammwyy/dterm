# Terminal

A clean, native terminal emulator for Linux, built with [CreamUI](https://github.com/sammwyy/creamui).
The executable is `dterm`; the desktop name is **Terminal**.

- One clean title for a single session; compact, closable tabs appear when you open another.
  Tabs can use an automatic shell/CWD title or a manual name.
- Split a tab horizontally or vertically, select and resize panes, and optionally broadcast
  keyboard input to every pane in the tab.
- UTF-8, ANSI/256/truecolor text, cursor addressing, alternate screen, configurable
  scrollback, text selection, clipboard copy/paste, and bracketed paste.
- Safe multi-line pastes ask for confirmation by default. Save the complete visible output and
  retained history to a file from the command palette.
- Scrollback search supports optional regex, match highlighting, previous/next navigation, and
  soft-wrapped Unicode text. Ctrl+click opens detected HTTP(S), `file://`, absolute, and `~/` paths.
- A command palette, configurable keyboard shortcuts, configurable control-island placement,
  terminal color themes, font selection/size, ligatures, and window opacity.
- Profiles can define an executable, arguments, initial directory, environment, and terminal
  color theme. New sessions use the selected profile.
- Optional session restoration reopens tabs and their working directories. Configuration changes
  made in the separate settings process apply live to open windows.
- Shells that emit OSC 133 markers support previous/next-command navigation and desktop
  notifications when a long-running command completes in an unfocused window.
- System theme, live CreamUI theme updates, and hybrid compositor decorations.

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
cargo run --locked -- --export-config /tmp/dterm.toml
cargo run --locked -- --import-config /tmp/dterm.toml
```

The current CreamUI checkout includes the native keyboard interception API required
for Tab and function keys to reach terminal applications.

## Shortcuts

All shortcuts are configurable in **Settings → Shortcuts**. `Alt+1` through `Alt+9` select tabs.

| Shortcut | Action |
| --- | --- |
| Ctrl+Shift+T / Ctrl+Shift+N / Ctrl+Shift+W | New tab / new window / close tab |
| Ctrl+Tab / Ctrl+Shift+Tab | Next / previous tab |
| Ctrl+Shift+C / Ctrl+Shift+V | Copy selection / paste |
| Ctrl+Shift+F | Search scrollback |
| Ctrl+Shift+P | Command palette |
| Ctrl+Shift+R / Ctrl+Shift+S | Rename tab / save terminal output |
| Ctrl+Alt+H / Ctrl+Alt+V / Ctrl+Alt+W | Split horizontally / vertically / close pane |
| Ctrl+Alt+] / Ctrl+Alt+[ | Focus next / previous pane |
| Ctrl+Alt+Plus / Ctrl+Alt+Minus | Grow / shrink the focused pane |
| Ctrl+Alt+B | Toggle input broadcast to all panes in the tab |
| Ctrl+Alt+Up / Ctrl+Alt+Down | Previous / next shell command marker |
| Ctrl+Comma | Open settings |
| Shift+PageUp / PageDown, mouse wheel | Scroll history |

Search opens the control island for the active tab and searches both the visible screen and retained
history. Type to search, toggle **.*** for regular expressions, and use **↑ / ↓** or
**Shift+Enter / Enter** to visit matches. Navigation wraps and the counter shows the current match
and total. Matches are highlighted, including text spanning soft-wrapped lines; invalid patterns
display an error. Search is case-sensitive (use `(?i)` in regex mode for case-insensitive matching).
**Escape** or **×** closes the bar and returns keyboard input to the terminal. Each tab keeps its own
query.

Applications using the alternate screen keep their own screen contents. Mouse reporting, hyperlinks,
and Sixel/Kitty image protocols are not implemented yet.

**Settings → General → Control island** controls whether floating search controls are enabled, their
horizontal or vertical layout, and independent horizontal (left, center, right) and vertical (top,
center, bottom) placement. Changes apply live after saving. Disabling it closes active searches and
ignores their activation shortcut; the terminal continues receiving normal input.
The island stays within the terminal area, clear of the header and status bar.
Vertical layouts scroll when the window is too short to fit every control.

## Configuration

`delib` resolves `$XDG_CONFIG_HOME/dterm/dterm.toml` (normally
`~/.config/dterm/dterm.toml`), followed by `$XDG_CONFIG_DIRS/dterm/dterm.toml`
for system defaults. A missing file uses the built-in defaults; settings writes
the user file atomically when you click **Save changes**. `--export-config` writes
the current resolved configuration, while `--import-config` validates a TOML file
before installing it as the user configuration.

```toml
shell = "bash"
font = "Fira Code"
profile = "default"
scrollback = 10000
confirm_multiline_paste = true
restore_session = true
notify_on_command_completion = true
command_completion_threshold_seconds = 10

[appearance]
font_size = 14
ligatures = true
opacity = 100

[profiles.default]
executable = "bash"
arguments = []
theme = "default" # default | dracula | solarized_dark | solarized_light

[profiles.work]
executable = "ssh"
arguments = ["dev.example"]
initial_directory = "Projects"
theme = "dracula"

[profiles.work.environment]
LANG = "C.UTF-8"

[island]
enabled = true
orientation = "horizontal" # horizontal | vertical
horizontal = "center" # left | center | right
vertical = "bottom" # top | center | bottom

[shortcuts]
new_tab = "Ctrl+Shift+T"
new_window = "Ctrl+Shift+N"
close_tab = "Ctrl+Shift+W"
next_tab = "Ctrl+Tab"
previous_tab = "Ctrl+Shift+Tab"
tab_1 = "Alt+1"
copy = "Ctrl+Shift+C"
paste = "Ctrl+Shift+V"
search = "Ctrl+Shift+F"
palette = "Ctrl+Shift+P"
rename_tab = "Ctrl+Shift+R"
save_output = "Ctrl+Shift+S"
split_horizontal = "Ctrl+Alt+H"
split_vertical = "Ctrl+Alt+V"
close_pane = "Ctrl+Alt+W"
focus_next_pane = "Ctrl+Alt+]"
focus_previous_pane = "Ctrl+Alt+["
grow_pane = "Ctrl+Alt+Plus"
shrink_pane = "Ctrl+Alt+Minus"
previous_command = "Ctrl+Alt+Up"
next_command = "Ctrl+Alt+Down"
toggle_broadcast_input = "Ctrl+Alt+B"
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

`src/island.rs` provides the shared surface and placement settings. Commands activate a per-session
`Mode` through `State::activate_mode`; `Escape` and the close control dismiss it. Search, paste
confirmation, the command palette, tab renaming, and output saving are island modes. New modes
supply their content and preferred size through `State::control_island`, and implement their state
updates in `Session::set_mode` / `refresh_mode` and keyboard handling in `State::key`. Keyboard
callbacks must remain independent of theme/context hooks; styling belongs to the window build.

```sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```

Licensed under [Apache-2.0](LICENSE).
