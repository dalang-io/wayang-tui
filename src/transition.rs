//! The cross-app **handoff frame**: `▸ launching <target>…`.
//!
//! Spec: `wayangos/docs/TUI-UX-REVAMP.md` §5b. When a HUD launches a sibling
//! tool (`wayang` → FIREWALL, a parent → a child) it must not drop back to the
//! normal screen between the two TUIs — that is the "flashblank". The parent
//! draws this one-line frame and keeps the alternate screen until the child
//! has drawn its own [splash](crate::splash); on return the parent draws it
//! again with a `loading …` action.
//!
//! Draw it with [`render`]; the line is vertically centred and cheap (one
//! `Paragraph`), so it can be shown immediately before spawning.

use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;
use crate::widgets::centered;

/// Draw `▸ <action> <target>…` centred in `area`.
///
/// * `action` — the verb: `"launching"` while handing off, `"loading"` while
///   returning / waiting for the child;
/// * `target` — the sibling tool or screen being handed to (`"wayang-fw"`).
///
/// The arrow is [`Ui::arrow`](crate::theme::Ui::arrow) (`▸`, or `>` with
/// `--plain`) and the ellipsis is `…` (`...` with `--plain`), so the frame is
/// correct in every theme.
pub fn render(f: &mut Frame, area: Rect, action: &str, target: &str, theme: &Theme) {
    let p = &theme.palette;
    let ellipsis = if theme.is_plain() { "..." } else { "…" };
    let line = Line::from(vec![
        Span::styled(format!("{} ", theme.ui.arrow()), p.bold(p.accent2)),
        Span::styled(format!("{action} "), p.bold(p.accent)),
        Span::styled(format!("{target}{ellipsis}"), p.fg(p.fg)),
    ]);
    let rect = centered(area, area.width, 1);
    f.render_widget(
        Paragraph::new(line)
            .alignment(Alignment::Center)
            .style(p.base()),
        rect,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Flags, Theme, WAYANG_FW};

    fn theme() -> Theme {
        Theme::from_env(WAYANG_FW, Flags::default(), false, Some("ansi"), None)
    }

    fn text(theme: &Theme, action: &str, target: &str) -> String {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 5)).unwrap();
        term.draw(|f| {
            let area = f.area();
            render(f, area, action, target, theme);
        })
        .unwrap();
        let buf = term.backend().buffer();
        (0..5)
            .flat_map(|y| (0..60).map(move |x| buf[(x, y)].symbol().to_string()))
            .collect()
    }

    #[test]
    fn handoff_says_launching_the_target() {
        let s = text(&theme(), "launching", "wayang-fw");
        assert!(s.contains("▸ launching wayang-fw…"), "{s:?}");
    }

    #[test]
    fn return_frame_says_loading_and_is_ascii_when_plain() {
        let plain = Theme::from_env(
            WAYANG_FW,
            Flags {
                plain: true,
                ..Flags::default()
            },
            false,
            Some("ansi"),
            None,
        );
        let s = text(&plain, "loading", "wayang-router");
        assert!(s.contains("> loading wayang-router..."), "{s:?}");
        assert!(!s.contains('▸'));
    }
}
