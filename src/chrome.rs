use crate::ui::*;
use creamui_core::BoxedWidget;
use creamui_reactive::Signal;
use creamui_render::{WindowDecorationMode, WindowHandle};
use std::{cell::RefCell, rc::Rc};
pub const HEADER: f32 = 44.0;
pub fn insets(width: f32, height: f32) -> (f32, f32) {
    let d = creamui_render::use_window_decorations();
    match d.mode {
        WindowDecorationMode::Client => (0.0, 112.0),
        WindowDecorationMode::Pending => (0.0, 128.0),
        WindowDecorationMode::Hybrid => {
            let c = d.controls;
            if c.width <= 0 || c.height <= 0 {
                return (0.0, 128.0);
            }
            if c.y as f32 >= height || (c.y + c.height) as f32 <= 0.0 {
                return (0.0, 0.0);
            }
            let left = c.x as f32;
            let right = left + c.width as f32;
            if (left + right) / 2.0 < width / 2.0 {
                ((right + 8.0).clamp(0.0, width), 0.0)
            } else {
                (0.0, (width - left + 8.0).clamp(0.0, width))
            }
        }
        _ => (0.0, 0.0),
    }
}
pub fn controls(
    window: &Rc<RefCell<Option<WindowHandle>>>,
    maximized: &Signal<bool>,
) -> Option<BoxedWidget> {
    if creamui_render::use_window_decorations().mode != WindowDecorationMode::Client {
        return None;
    }
    let mut children = Vec::new();
    for (i, label) in ["−", "□", "×"].into_iter().enumerate() {
        let window = window.clone();
        let max = maximized.clone();
        children.push(button(label, 32.0, move || {
            if let Some(w) = window.borrow().as_ref() {
                match i {
                    0 => w.minimize(),
                    1 => {
                        let next = !max.peek();
                        w.set_maximized(next);
                        max.set(next);
                    }
                    _ => w.close(),
                }
            }
        }));
    }
    Some(view(row(4.0), children))
}

/// Center on the entire window, shrinking or clamping only to avoid native controls.
pub fn centered(width: f32, start: f32, end: f32, wanted: f32) -> (f32, f32) {
    let left = start + 8.0;
    let right = (width - end - 8.0).max(left);
    let middle = width / 2.0;
    let symmetric = 2.0 * (middle - left).min(right - middle).max(0.0);
    let actual = wanted
        .min(if symmetric >= 160.0 {
            symmetric
        } else {
            right - left
        })
        .max(0.0);
    (
        (middle - actual / 2.0).clamp(left, (right - actual).max(left)),
        actual,
    )
}
