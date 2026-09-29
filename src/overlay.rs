//! A minimal shared frame for modal overlays: help (`?`), REVIEW and
//! quick-jump (`/`).
//!
//! The products differ in *content*, not chrome, so this only owns the chrome:
//! a centered, focused `◢ TITLE ◣` panel, a scrollable content region and a
//! one-row footer of keycaps. Callers render lists/diffs/help into
//! [`OverlayAreas::content`] and keep the footer for hints.
//!
//! ```no_run
//! # use ratatui::Frame;
//! # fn demo(f: &mut Frame, t: &wayang_tui::theme::Theme) {
//! use wayang_tui::overlay::Overlay;
//! use ratatui::widgets::{Paragraph, Widget};
//!
//! let ov = Overlay::new("HELP").keys(vec![("j/k", "scroll"), ("esc", "close")]);
//! let areas = ov.render(f, f.area(), t);
//! Paragraph::new("…keys…").render(areas.content, f.buffer_mut());
//! # }
//! ```

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;
use crate::widgets::{centered, keycaps, panel_focused};

/// Default overlay width in columns.
pub const DEFAULT_WIDTH: u16 = 72;
/// Default overlay height in rows.
pub const DEFAULT_HEIGHT: u16 = 20;

/// The overlay chrome: title, footer keys and a preferred size. Chainable.
#[derive(Debug, Clone)]
pub struct Overlay<'a> {
    pub title: &'a str,
    pub keys: Vec<(&'a str, &'a str)>,
    pub width: u16,
    pub height: u16,
}

impl<'a> Overlay<'a> {
    /// A titled overlay at [`DEFAULT_WIDTH`] × [`DEFAULT_HEIGHT`].
    pub fn new(title: &'a str) -> Self {
        Overlay {
            title,
            keys: Vec::new(),
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
        }
    }

    /// Footer keycaps, e.g. `[("j/k", "scroll"), ("esc", "close")]`.
    pub fn keys(mut self, keys: Vec<(&'a str, &'a str)>) -> Self {
        self.keys = keys;
        self
    }

    /// Preferred size, clamped to the viewport when rendered.
    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Draw the chrome centered in `viewport` and return the regions to paint.
    ///
    /// The panel is always the focused one (a modal takes focus); `content` is
    /// the scroll region and `footer` the keycap row (`None` when no keys or
    /// the overlay is a single row).
    pub fn render(&self, f: &mut Frame, viewport: Rect, theme: &Theme) -> OverlayAreas {
        let outer = centered(viewport, self.width, self.height);
        let inner = panel_focused(f, outer, self.title, None, theme);
        let (content, footer) = if self.keys.is_empty() || inner.height < 2 {
            (inner, None)
        } else {
            let [c, ft] =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);
            (c, Some(ft))
        };
        if let Some(ft) = footer {
            f.render_widget(Paragraph::new(keycaps(&self.keys, theme)), ft);
        }
        OverlayAreas {
            outer,
            content,
            footer,
        }
    }
}

/// Regions produced by [`Overlay::render`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayAreas {
    /// The panel's outer rectangle (including the frame).
    pub outer: Rect,
    /// The scrollable content region, inside the frame above the footer.
    pub content: Rect,
    /// The footer keycap row, if the overlay has keys and room for one.
    pub footer: Option<Rect>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Flags, WAYANG_FW};

    fn theme() -> Theme {
        Theme::from_env(WAYANG_FW, Flags::default(), false, Some("ansi"), None)
    }

    #[test]
    fn render_centers_and_reserves_a_footer() {
        let t = theme();
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 40)).unwrap();
        let ov = Overlay::new("HELP").keys(vec![("j/k", "scroll"), ("esc", "close")]);
        let mut areas = None;
        term.draw(|f| {
            let viewport = f.area();
            areas = Some(ov.render(f, viewport, &t));
        })
        .unwrap();
        let a = areas.unwrap();
        assert_eq!(a.outer, Rect::new(14, 10, 72, 20));
        assert!(a.footer.is_some());
        assert_eq!(a.content.height + 1 + 2, a.outer.height, "footer + frame");
        let text: String = {
            let buf = term.backend().buffer();
            (0..40)
                .flat_map(|y| (0..100).map(move |x| buf[(x, y)].symbol().to_string()))
                .collect()
        };
        assert!(text.contains("▸ HELP"), "focused title: {text:?}");
        assert!(text.contains("scroll"));
    }

    #[test]
    fn no_keys_means_no_footer() {
        let t = theme();
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 20)).unwrap();
        let ov = Overlay::new("REVIEW");
        let mut areas = None;
        term.draw(|f| {
            let viewport = f.area();
            areas = Some(ov.render(f, viewport, &t));
        })
        .unwrap();
        assert_eq!(areas.unwrap().footer, None);
    }
}
