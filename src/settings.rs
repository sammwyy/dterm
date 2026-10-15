use crate::{
    chrome,
    config::Config,
    shortcuts::{Action, Binding},
    ui::*,
};
use creamui_core::{
    layout::{Dimension, Style},
    BoxedWidget, LinearGradient, Size, StateStyle, Styled,
};
use creamui_reactive::Signal;
use creamui_render::{
    platform::{Key, KeyEvent, Modifiers},
    WindowHandle,
};
use creamui_theme::use_theme;
use creamui_widgets::{
    Button, ButtonSize, ButtonState, ButtonVariant, CUIWindowDragArea, Icon, RawButton,
    RawScrollView, RawView, ScrollController, Symbol, Tab, TabColors, Tabs, Text, TextInput,
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    General,
    Shortcuts,
    About,
}
#[derive(Clone)]
pub struct State {
    pub window: Rc<RefCell<Option<WindowHandle>>>,
    maximized: Signal<bool>,
    scroll: ScrollController,
    page: Signal<Page>,
    config: Signal<Config>,
    saved: Signal<Config>,
    capturing: Signal<Option<Action>>,
    message: Signal<Option<String>>,
    path: PathBuf,
}
impl State {
    pub fn new() -> Self {
        let (config, message) = match Config::load() {
            Ok(config) => (config, None),
            Err(error) => (Config::default(), Some(error)),
        };
        Self {
            window: Rc::new(RefCell::new(None)),
            maximized: Signal::new(false),
            scroll: ScrollController::new(0.0),
            page: Signal::new(Page::General),
            saved: Signal::new(config.clone()),
            config: Signal::new(config),
            capturing: Signal::new(None),
            message: Signal::new(message),
            path: crate::config::path(),
        }
    }
    fn save(&self) {
        let config = self.config.peek();
        match config.save_to(&self.path) {
            Ok(()) => {
                self.saved.set(config);
                self.capturing.set(None);
                self.message.set(Some("Changes saved.".into()));
            }
            Err(error) => self
                .message
                .set(Some(format!("Couldn't save settings: {error}"))),
        }
    }
    pub fn key(&self, event: KeyEvent, modifiers: Modifiers) -> bool {
        let Some(action) = self.capturing.peek() else {
            return false;
        };
        if !event.pressed {
            return true;
        }
        match event.key {
            Key::Escape => {
                self.capturing.set(None);
                self.message.set(None);
            }
            Key::Backspace | Key::Delete if modifiers == Modifiers::default() => {
                self.config.update(|c| {
                    c.shortcuts.insert(action.id(), String::new());
                });
                self.capturing.set(None);
                self.message.set(None);
            }
            _ => {
                if let Some(binding) = Binding::from_event(&event.key, modifiers) {
                    let accelerator = binding.display();
                    let mut config = self.config.peek();
                    config.shortcuts.insert(action.id(), accelerator);
                    match config.bindings() {
                        Ok(_) => {
                            self.config.set(config);
                            self.capturing.set(None);
                            self.message.set(None);
                        }
                        Err(error) => self.message.set(Some(error)),
                    }
                }
            }
        }
        true
    }
    pub fn build(&self, size: Size) -> BoxedWidget {
        let theme = use_theme();
        let page = self.page.get();
        let body = match page {
            Page::General => self.general(),
            Page::Shortcuts => self.shortcuts(),
            Page::About => crate::about::build("Terminal"),
        };
        let inset = 24.0 + ((size.width - 720.0) / 2.0).max(0.0);
        Box::new(
            RawView::new(Style {
                size: creamui_core::layout::Size {
                    width: Dimension::Length(size.width),
                    height: Dimension::Length(size.height),
                },
                ..column(0.0)
            })
            .background(LinearGradient::new(
                150.0,
                theme.colors.surface,
                theme.colors.surface.mix(theme.colors.accent, 0.055),
            ))
            .with_children(vec![
                self.header(size.width),
                Box::new(
                    RawScrollView::controlled(
                        Style {
                            flex_grow: 1.0,
                            flex_basis: Dimension::Length(0.0),
                            ..column(0.0)
                        },
                        self.scroll.clone(),
                    )
                    .scrollbar_width(5.0)
                    .child(view(pad(column(0.0), inset, 24.0), vec![body])),
                ),
            ]),
        )
    }
    fn header(&self, width: f32) -> BoxedWidget {
        let theme = use_theme();
        let (start, end) = chrome::insets(width, 64.0);
        let (x, available) = chrome::centered(width, start, end, 324.0);
        let tab_width = ((available - 24.0) / 3.0).max(0.0);
        let mut colors = TabColors::dark();
        colors.background = theme.colors.surface_elevated;
        colors.active_background = theme.colors.selection_background;
        colors.active_text = theme.colors.accent;
        colors.container_radius = 12.0;
        colors.inactive_background = None;
        let mut tabs = Tabs::new(colors, pad(row(8.0), 4.0, 4.0));
        for (page, label) in [
            (Page::General, "General"),
            (Page::Shortcuts, "Shortcuts"),
            (Page::About, "About"),
        ] {
            let state = self.clone();
            tabs = tabs.child(Box::new(Tab::new(
                colors,
                fixed(tab_width, 34.0),
                label,
                self.page.get() == page,
                move || {
                    state.capturing.set(None);
                    state.scroll.set(0.0);
                    state.page.set(page);
                },
            )));
        }
        let client = chrome::controls(&self.window, &self.maximized);
        let trailing = if client.is_some() { 112.0 } else { 0.0 };
        let mut children = vec![
            view(fixed(x, 1.0), vec![]),
            view(fixed(available, 42.0), vec![Box::new(tabs)]),
            view(
                fixed((width - x - available - trailing).max(0.0), 1.0),
                vec![],
            ),
        ];
        if let Some(controls) = client {
            children.push(controls);
            children.push(view(fixed(8.0, 1.0), vec![]));
        }
        Box::new(
            CUIWindowDragArea::new()
                .layout(fixed(width, 64.0))
                .with_children(children),
        )
    }

    fn general(&self) -> BoxedWidget {
        let state = self.clone();
        view(column(22.0),vec![
            heading("General","Choose the shell for new terminal sessions."),
            Box::new(RawView::new(pad(column(12.0),16.0,16.0)).background(use_theme().colors.surface_elevated).corner_radius(14.0).outline(use_theme().colors.border,1.0)
                .with_children(vec![
                    Box::new(Text::new("Default shell").font_size(13.0).bold(true)),
                    Box::new(TextInput::new(self.config.get().shell,move|shell|state.config.update(|c|c.shell=shell)).placeholder("bash")),
                    Box::new(Text::secondary("If the executable is unavailable, Terminal tries sh, then bash.").font_size(12.0)),
                ])),
            self.actions(Page::General),
        ])
    }
    fn shortcuts(&self) -> BoxedWidget {
        let all = Action::all();
        let mut children = vec![heading(
            "Keyboard shortcuts",
            "Click a shortcut and press your keys. Escape cancels; Backspace clears.",
        )];
        for (label, actions) in [
            ("Windows & tabs", all[..5].to_vec()),
            ("Jump to a tab", all[5..14].to_vec()),
            ("Terminal", all[14..].to_vec()),
        ] {
            let mut rows = Vec::new();
            for (i, action) in actions.into_iter().enumerate() {
                if i > 0 {
                    rows.push(Box::new(
                        RawView::new(Style {
                            size: creamui_core::layout::Size {
                                width: Dimension::Percent(1.0),
                                height: Dimension::Length(1.0),
                            },
                            flex_shrink: 0.0,
                            ..Default::default()
                        })
                        .background(use_theme().colors.border),
                    ) as BoxedWidget);
                }
                rows.push(self.shortcut_row(action));
            }
            children.push(view(
                column(8.0),
                vec![
                    Box::new(Text::secondary(label).font_size(12.0)),
                    Box::new(
                        RawView::new(column(0.0))
                            .background(use_theme().colors.surface_elevated)
                            .corner_radius(14.0)
                            .outline(use_theme().colors.border, 1.0)
                            .with_children(rows),
                    ),
                ],
            ));
        }
        children.push(self.actions(Page::Shortcuts));
        view(column(24.0), children)
    }
    fn shortcut_row(&self, action: Action) -> BoxedWidget {
        let theme = use_theme();
        let capturing = self.capturing.get() == Some(action);
        let accelerator = self
            .config
            .get()
            .shortcuts
            .get(&action.id())
            .cloned()
            .unwrap_or_default();
        let label = if capturing {
            "Press shortcut…".into()
        } else if accelerator.is_empty() {
            "Disabled".into()
        } else {
            accelerator.split('+').collect::<Vec<_>>().join(" + ")
        };
        let state = self.clone();
        let row = view(
            pad(row(12.0), 16.0, 10.5),
            vec![
                Box::new(
                    RawView::new(Style {
                        justify_content: Some(creamui_core::layout::JustifyContent::Center),
                        ..fixed(24.0, 24.0)
                    })
                    .background(creamui_theme::Color::rgb(207, 91, 36))
                    .corner_radius(8.0)
                    .child(Box::new(
                        Icon::new(Symbol::Keyboard, creamui_theme::Color::rgb(255, 255, 255))
                            .size(14.0),
                    )),
                ),
                Box::new(Text::new(action.label()).font_size(13.0).layout(Style {
                    flex_grow: 1.0,
                    flex_basis: Dimension::Length(0.0),
                    ..Default::default()
                })),
                Box::new(
                    RawButton::new(
                        Style {
                            justify_content: Some(creamui_core::layout::JustifyContent::Center),
                            ..fixed(156.0, 34.0)
                        },
                        move || {
                            state.capturing.set(Some(action));
                            state.message.set(None);
                        },
                    )
                    .background(if capturing {
                        theme.colors.selection_background
                    } else {
                        CLEAR
                    })
                    .corner_radius(8.0)
                    .hover_style(StateStyle::new().background(theme.colors.surface_hover))
                    .focus_style(StateStyle::new().outline(theme.colors.accent, 1.0))
                    .child(Box::new(Text::secondary(label).font_size(12.0))),
                ),
            ],
        );
        let mut children = vec![row];
        if capturing {
            if let Some(message) = self.message.get() {
                children.push(Box::new(
                    Text::new(message)
                        .font_size(12.0)
                        .color(theme.colors.danger)
                        .layout(pad(Style::default(), 16.0, 4.0)),
                ));
            }
        }
        view(column(0.0), children)
    }
    fn actions(&self, page: Page) -> BoxedWidget {
        let reset = self.clone();
        let save = self.clone();
        let mut buttons: Vec<BoxedWidget> = vec![Box::new(Button::styled(
            ButtonVariant::Secondary,
            ButtonSize::Sm,
            "Restore defaults",
            ButtonState::Normal,
            move || {
                reset.capturing.set(None);
                reset.message.set(None);
                reset.config.update(|c| {
                    if page == Page::General {
                        c.shell = Config::default().shell
                    } else {
                        c.shortcuts = Config::default().shortcuts
                    }
                });
            },
        ))];
        if self.config.get() != self.saved.get() {
            buttons.push(Box::new(Button::styled(
                ButtonVariant::Primary,
                ButtonSize::Sm,
                "Save changes",
                ButtonState::Normal,
                move || save.save(),
            )));
        }
        let mut children = vec![view(
            Style {
                justify_content: Some(creamui_core::layout::JustifyContent::End),
                ..row(8.0)
            },
            buttons,
        )];
        if let Some(message) = self.message.get() {
            children.push(Box::new(Text::secondary(message).font_size(12.0)));
        }
        view(column(12.0), children)
    }
}
fn heading(title: &str, subtitle: &str) -> BoxedWidget {
    view(
        column(6.0),
        vec![
            Box::new(Text::new(title).font_size(24.0).bold(true)),
            Box::new(Text::secondary(subtitle).font_size(13.0)),
        ],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use creamui_core::Renderer;
    use creamui_render::{Damage, Rasterizer, SceneRecorder, WindowDecorations};
    use creamui_theme::{Theme, ThemeProvider};
    #[test]
    fn recorder_detects_conflicts_cancels_clears_and_saves_atomically() {
        creamui_reactive::with_context_scope(|| {
            let theme = Theme::light();
            creamui_reactive::provide_context(ThemeProvider::new(theme));
            creamui_reactive::provide_context(Signal::new(WindowDecorations::default()));
            let root = std::env::temp_dir().join(format!(
                "dterm-settings-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let mut state = State::new();
            state.path = root.join("dterm.toml");
            state.config.set(Config::default());
            state.saved.set(Config::default());
            state.page.set(Page::Shortcuts);
            let size = Size {
                width: 720.0,
                height: 640.0,
            };
            let mut recorder = SceneRecorder::new();
            recorder.begin(720, 640, 1.0, CLEAR, theme.colors);
            let scene = Renderer::new().render(state.build(size), size, &mut recorder);
            scene
                .on_key_at(3)
                .expect("first shortcut is keyboard accessible")(
                creamui_core::KeyInput {
                    key: creamui_core::Key::Enter,
                    modifiers: Default::default(),
                },
            );
            assert_eq!(state.capturing.peek(), Some(Action::NewTab));
            let event = |key| KeyEvent {
                key,
                pressed: true,
                synthetic: false,
            };
            state.key(
                event(Key::Character("T".into())),
                Modifiers {
                    ctrl: true,
                    alt: true,
                    ..Default::default()
                },
            );
            assert_eq!(state.config.peek().shortcuts["new_tab"], "Ctrl+Alt+T");
            assert!(
                !state.path.exists(),
                "recording is a draft until Save changes"
            );
            state.capturing.set(Some(Action::NewTab));
            state.key(
                event(Key::Character("N".into())),
                Modifiers {
                    ctrl: true,
                    shift: true,
                    ..Default::default()
                },
            );
            assert_eq!(state.capturing.peek(), Some(Action::NewTab));
            assert!(state.message.peek().unwrap().contains("New window"));
            state.key(event(Key::Escape), Modifiers::default());
            assert!(state.capturing.peek().is_none());
            assert_eq!(state.config.peek().shortcuts["new_tab"], "Ctrl+Alt+T");
            state.capturing.set(Some(Action::NewTab));
            state.key(event(Key::Backspace), Modifiers::default());
            assert_eq!(state.config.peek().shortcuts["new_tab"], "");
            state.config.update(|c| c.shell = "zsh".into());
            state.save();
            assert_eq!(
                Config::parse(&std::fs::read_to_string(&state.path).unwrap()).unwrap(),
                state.config.peek()
            );
            assert_eq!(state.saved.peek(), state.config.peek());
            assert!(!state.key(event(Key::Tab), Modifiers::default()));
            std::fs::remove_dir_all(root).unwrap();
        });
    }
    #[test]
    fn settings_tabs_are_centered_and_shortcut_cards_render_at_small_sizes() {
        use creamui_core::Point;
        for (appearance, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
            creamui_reactive::with_context_scope(|| {
                creamui_reactive::provide_context(ThemeProvider::new(theme));
                creamui_reactive::provide_context(Signal::new(WindowDecorations::default()));
                let state = State::new();
                state.config.set(Config::default());
                state.saved.set(Config::default());
                for (page, name) in [(Page::General, "general"), (Page::Shortcuts, "shortcuts")] {
                    state.page.set(page);
                    for (width, height) in [(720, 640), (480, 420)] {
                        let size = Size {
                            width: width as f32,
                            height: height as f32,
                        };
                        let mut recorder = SceneRecorder::new();
                        recorder.begin(width, height, 1.0, CLEAR, theme.colors);
                        let scene = Renderer::new().render(state.build(size), size, &mut recorder);
                        let xs = (0..width)
                            .filter(|x| {
                                scene
                                    .focus_hit_test(Point {
                                        x: *x as f32,
                                        y: 32.0,
                                    })
                                    .is_some()
                            })
                            .collect::<Vec<_>>();
                        assert!(
                            ((xs[0] + xs[xs.len() - 1]) as f32 / 2.0 - width as f32 / 2.0).abs()
                                < 2.0
                        );
                        assert!(scene.focus_id_at(3).is_some());
                        if let Some(dir) = std::env::var_os("DTERM_SNAPSHOT_DIR") {
                            let mut raster = Rasterizer::new(width, height);
                            raster.render(&recorder.finish(), &Damage::Full);
                            raster
                                .pixmap()
                                .save_png(
                                    std::path::PathBuf::from(dir)
                                        .join(format!("{appearance}-{name}-{width}.png")),
                                )
                                .unwrap();
                        }
                    }
                }
            });
        }
    }

    #[test]
    fn about_fits_and_links_remain_accessible_in_a_compact_window() {
        for (appearance, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
            creamui_reactive::with_context_scope(|| {
                creamui_reactive::provide_context(ThemeProvider::new(theme));
                creamui_reactive::provide_context(Signal::new(WindowDecorations::default()));
                let state = State::new();
                state.page.set(Page::About);
                for (width, height) in [(720, 640), (480, 420)] {
                    let mut recorder = SceneRecorder::new();
                    recorder.begin(width, height, 1.0, CLEAR, theme.colors);
                    let size = Size {
                        width: width as f32,
                        height: height as f32,
                    };
                    let scene = Renderer::new().render(state.build(size), size, &mut recorder);
                    assert!(scene.focus_id_at(0).is_some());
                    if height == 640 {
                        assert!(scene.focus_id_at(6).is_some());
                    }
                    if let Some(dir) = std::env::var_os("DTERM_SNAPSHOT_DIR") {
                        let list = recorder.finish();
                        let mut raster = Rasterizer::new(width, height);
                        raster.render(&list, &Damage::Full);
                        raster
                            .pixmap()
                            .save_png(
                                std::path::PathBuf::from(dir)
                                    .join(format!("{appearance}-about-{width}.png")),
                            )
                            .unwrap();
                    }
                }
            });
        }
    }
}
