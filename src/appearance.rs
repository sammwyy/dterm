use creamui_render::{AppHandle, WindowHandle};
use creamui_theme::Theme;
use creamui_theme_watch::ThemeListener;
use std::{
    cell::RefCell,
    fs,
    rc::Rc,
    sync::{Arc, Mutex},
};

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

pub fn watch_theme_reloads(
    app: AppHandle,
    listener: Arc<Mutex<ThemeListener>>,
    window: Rc<RefCell<Option<WindowHandle>>>,
) {
    let next_app = app.clone();
    let next_listener = listener.clone();
    let next_window = window.clone();
    app.spawn_background(
        move || listener.lock().is_ok_and(|listener| listener.recv()),
        move |changed| {
            if changed {
                if let Some(window) = window.borrow().as_ref() {
                    window.set_theme(load_system_theme());
                }
                watch_theme_reloads(next_app, next_listener, next_window);
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
