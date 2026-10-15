use creamui_render::{AppHandle, WindowHandle};
use creamui_theme::Theme;
use dbus::{blocking::Connection, message::MatchRule};
use std::{
    cell::RefCell,
    fs,
    rc::Rc,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
const THEME_PATH: &str = "/org/creamui/Theme";
const THEME_INTERFACE: &str = "org.creamui.Theme";
const THEME_RELOAD: &str = "ReloadTheme";
pub fn load_system_theme() -> Theme {
    match creamui_theme_loader::SystemThemeLoader::new().load() {
        Ok(appearance) => {
            if let Some(font_family) = &appearance.font_family {
                creamui_fonts::use_system_font(font_family);
            }
            appearance.theme
        }
        Err(error) => {
            eprintln!("terminal: failed to load CreamUI system theme: {error}");
            Theme::light()
        }
    }
}

/// Coconut broadcasts this signal after Settings persists a new CreamUI
/// appearance. Keep the receiver on its own D-Bus connection so it cannot
/// block the native UI event loop.
pub fn listen_for_theme_reloads() -> Receiver<()> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let Ok(connection) = Connection::new_session() else {
            eprintln!("terminal: session bus unavailable; live theme updates are disabled");
            return;
        };
        if let Err(error) = connection.add_match(
            MatchRule::new_signal(THEME_INTERFACE, THEME_RELOAD).with_path(THEME_PATH),
            move |_: (), _, _| sender.send(()).is_ok(),
        ) {
            eprintln!("terminal: failed to listen for CreamUI theme updates: {error}");
            return;
        }
        loop {
            if let Err(error) = connection.process(Duration::from_secs(1)) {
                eprintln!("terminal: CreamUI theme listener stopped: {error}");
                return;
            }
        }
    });
    receiver
}

pub fn watch_theme_reloads(
    app: AppHandle,
    events: Arc<Mutex<Receiver<()>>>,
    window: Rc<RefCell<Option<WindowHandle>>>,
) {
    let next_app = app.clone();
    let next_events = events.clone();
    let next_window = window.clone();
    app.spawn_background(
        move || events.lock().is_ok_and(|events| events.recv().is_ok()),
        move |changed| {
            if changed {
                if let Some(window) = window.borrow().as_ref() {
                    window.set_theme(load_system_theme());
                }
                watch_theme_reloads(next_app, next_events, next_window);
            }
        },
    );
}
pub fn ensure_wayland_display() {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return;
    }
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };
    use std::os::unix::fs::FileTypeExt;
    let mut sockets = fs::read_dir(runtime)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_socket()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("wayland-"))
        .collect::<Vec<_>>();
    sockets.sort();
    if let Some(socket) = sockets.first() {
        std::env::set_var("WAYLAND_DISPLAY", socket);
    }
}
