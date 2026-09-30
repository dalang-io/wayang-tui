//! The focused-pane model and the colour-free focus marker.
//!
//! Spec: `wayangos/docs/TUI-UX-REVAMP.md` §5, "Focused-pane highlight".
//! Exactly one pane owns the keyboard; it is unmistakable **without colour**:
//!
//! * the focused pane's border + title render in `accent`, unfocused panes in
//!   `border`/`dim`;
//! * the focused title is prefixed [`Theme::focus_mark`] (`▸`, or `>` in
//!   `--plain`/mono);
//! * a focused list shows the selection row moving with `↑↓`; a focused form
//!   shows the field cursor.
//!
//! [`Focus`] is the per-pane state, [`FocusRing`] tracks which of `n` panes is
//! focused and cycles with `Tab`/`BackTab`.

use ratatui::style::Style;

use crate::theme::Theme;

/// Whether a pane owns the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Focused,
    Unfocused,
}

impl Focus {
    pub const fn is_focused(self) -> bool {
        matches!(self, Focus::Focused)
    }

    /// The colour-free marker: `▸` (`>` in plain/mono) when focused, `""` when
    /// not. Never colour-only.
    pub fn marker(self, theme: &Theme) -> &'static str {
        if self.is_focused() {
            theme.focus_mark()
        } else {
            ""
        }
    }

    /// [`Focus::marker`] with a trailing space, or empty — ready to prefix a
    /// title.
    pub fn prefix(self, theme: &Theme) -> String {
        match self.marker(theme) {
            "" => String::new(),
            m => format!("{m} "),
        }
    }

    /// Title style: `accent` + bold when focused, `dim` otherwise.
    pub fn title_style(self, theme: &Theme) -> Style {
        if self.is_focused() {
            theme.palette.bold(theme.palette.accent)
        } else {
            theme.palette.fg(theme.palette.dim)
        }
    }
}

impl From<bool> for Focus {
    fn from(focused: bool) -> Self {
        if focused {
            Focus::Focused
        } else {
            Focus::Unfocused
        }
    }
}

/// Which of `n` panes owns the keyboard. `Tab` advances, `BackTab` retreats,
/// both wrapping. Zero panes is a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusRing {
    len: usize,
    current: usize,
}

impl FocusRing {
    /// A ring over `len` panes, starting at pane 0.
    pub fn new(len: usize) -> Self {
        FocusRing { len, current: 0 }
    }

    /// Number of panes in the ring.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Index of the focused pane (0 when the ring is empty).
    pub fn current(&self) -> usize {
        self.current
    }

    /// Focus pane `i` (clamped to the ring).
    pub fn set(&mut self, i: usize) {
        if self.len > 0 {
            self.current = i.min(self.len - 1);
        }
    }

    /// Advance by one (`Tab`), wrapping.
    pub fn next(&mut self) {
        if self.len > 0 {
            self.current = (self.current + 1) % self.len;
        }
    }

    /// Retreat by one (`BackTab`), wrapping.
    pub fn prev(&mut self) {
        if self.len > 0 {
            self.current = (self.current + self.len - 1) % self.len;
        }
    }

    /// Is pane `i` the focused one?
    pub fn is_focused(&self, i: usize) -> bool {
        self.len > 0 && i == self.current
    }

    /// The [`Focus`] state of pane `i`.
    pub fn focus(&self, i: usize) -> Focus {
        Focus::from(self.is_focused(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Flags, WAYANG_FW};

    fn mono() -> Theme {
        Theme::from_env(WAYANG_FW, Flags::default(), true, None, None)
    }

    fn plain() -> Theme {
        Theme::from_env(
            WAYANG_FW,
            Flags {
                plain: true,
                ..Flags::default()
            },
            false,
            Some("ansi"),
            None,
        )
    }

    fn fancy() -> Theme {
        Theme::from_env(WAYANG_FW, Flags::default(), false, None, Some("24bit"))
    }

    #[test]
    fn marker_is_glyph_not_colour() {
        assert_eq!(Focus::Focused.marker(&fancy()), "▸");
        assert_eq!(Focus::Unfocused.marker(&fancy()), "");
        // Mono keeps `▸` too (colour-free); only `--plain` uses `>`.
        assert_eq!(Focus::Focused.marker(&mono()), "▸");
        assert_eq!(Focus::Focused.marker(&plain()), ">");
        assert_eq!(Focus::Focused.prefix(&fancy()), "▸ ");
        assert_eq!(Focus::Unfocused.prefix(&fancy()), "");
    }

    #[test]
    fn ring_cycles_and_wraps() {
        let mut r = FocusRing::new(3);
        assert!(r.is_focused(0));
        assert_eq!(r.focus(1), Focus::Unfocused);
        r.next();
        assert!(r.is_focused(1));
        r.next();
        r.next();
        assert_eq!(r.current(), 0, "wraps forward");
        r.prev();
        assert!(r.is_focused(2), "wraps backward");
        r.set(9);
        assert_eq!(r.current(), 2, "set clamps");
    }

    #[test]
    fn empty_ring_is_a_noop() {
        let mut r = FocusRing::new(0);
        r.next();
        r.prev();
        r.set(2);
        assert_eq!(r.current(), 0);
        assert!(!r.is_focused(0));
    }
}
