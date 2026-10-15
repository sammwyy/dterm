mod about;
mod app;
mod appearance;
mod chrome;
mod cli;
mod config;
mod input;
mod island;
mod screen;
mod search;
mod settings;
mod shortcuts;
mod terminal;
mod terminal_graphics;
mod ui;

use creamui_render::{AppBuilder, BlurRegion, CompositorIntegrationRequest, WindowOptions};
use creamui_theme_watch::ThemeListener;
use std::sync::{mpsc, Arc, Mutex};

fn main() -> Result<(), String> {
    let launch = cli::parse(std::env::args_os().skip(1))?;
    match launch {
        cli::Launch::Help => println!("{}", cli::HELP),
        cli::Launch::Version => println!("Terminal {}", env!("CARGO_PKG_VERSION")),
        cli::Launch::Settings => run_settings(),
        cli::Launch::Terminal { cwd, command } => run_terminal(cwd, command),
    }
    Ok(())
}
fn options(title: &str, width: u32, height: u32) -> WindowOptions {
    WindowOptions {
        title: title.into(),
        width,
        height,
        transparent: true,
        decorations: true,
        blur: Some(BlurRegion::Window),
        theme: appearance::load_system_theme(),
        ..Default::default()
    }
}
fn run_terminal(cwd: std::path::PathBuf, command: Vec<String>) {
    appearance::ensure_wayland_display();
    let configuration = config::Config::load();
    let (sender, receiver) = mpsc::sync_channel(128);
    let state = app::State::new(cwd, command, sender.clone());
    if let Ok(config) = &configuration {
        state.configure(config.clone());
    }
    state.add();
    if let Err(error) = configuration {
        state.show_error(error);
    }
    app::watch_configuration(sender);
    let cleanup = state.clone();
    let window = state.window.clone();
    let ready = window.clone();
    let events = Arc::new(Mutex::new(receiver));
    let theme_events = Arc::new(Mutex::new(ThemeListener::start()));
    let opts = options("Terminal", 1000, 680);
    AppBuilder::new()
        .on_started(move |app| {
            let keys = state.clone();
            let build = state.clone();
            app.append_window(
                opts,
                ui::CLEAR,
                move |handle| {
                    handle.set_compositor_integration(Some(CompositorIntegrationRequest::Hybrid));
                    handle.on_keyboard_input(move |event, modifiers| keys.key(event, modifiers));
                    *ready.borrow_mut() = Some(handle);
                },
                move |size| build.build(size),
            );
            app::watch(app.clone(), events, state);
            appearance::watch_theme_reloads(app, theme_events, window);
        })
        .run();
    cleanup.cleanup();
}
fn run_settings() {
    appearance::ensure_wayland_display();
    let state = settings::State::new();
    let window = state.window.clone();
    let ready = window.clone();
    let keys = state.clone();
    let theme_events = Arc::new(Mutex::new(ThemeListener::start()));
    let opts = options("Terminal Settings", 720, 640);
    AppBuilder::new()
        .on_started(move |app| {
            app.append_window(
                opts,
                ui::CLEAR,
                move |handle| {
                    handle.set_compositor_integration(Some(CompositorIntegrationRequest::Hybrid));
                    handle.on_keyboard_input(move |event, modifiers| keys.key(event, modifiers));
                    *ready.borrow_mut() = Some(handle);
                },
                move |size| state.build(size),
            );
            appearance::watch_theme_reloads(app, theme_events, window);
        })
        .run();
}
