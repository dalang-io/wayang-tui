//! The first-frame **splash**: what a HUD draws the instant it owns the
//! terminal, before any data has been sampled.
//!
//! Spec: `wayangos/docs/TUI-UX-REVAMP.md` §5b. Entering the alternate screen
//! and then drawing nothing shows an empty (light/SSH → white) frame for a
//! beat. The fix is a rule, not per-product polish: the very first draw after
//! [`crate::term::TermGuard::enter`] is this loading frame — brand logotype +
//! `loading <tool>…` + spinner + breadcrumb — and only then does sampling run.
//!
//! It never blocks: the caller draws it, then samples, then replaces it. The
//! same call frames a return from a child tool (see [`crate::transition`]).

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme::Theme;
use crate::widgets::centered;

/// The loading ellipsis for the active glyph set.
fn ellipsis(theme: &Theme) -> &'static str {
    if theme.is_plain() { "..." } else { "…" }
}

/// Draw a **full-screen** splash into `area`: a frame across the whole terminal
/// with the brand logotype (scaled to the terminal height) centred inside, the
/// `loading <tool>…` spinner and the breadcrumb.
///
/// * `tool` — the tool being started (`"wayang-fw"`), shown as `loading <tool>…`;
/// * `subtitle` — optional context line (e.g. `FIREWALL SYSTEM`), shown as the
///   panel's right title;
/// * `tick` — the animation tick; selected through
///   [`Ui::spinner`](crate::theme::Ui::spinner) so it works with `--plain`.
///
/// The composition is centred inside the frame, so the splash fills the screen
/// at any size and is drawable before data exists.
pub fn render(
    f: &mut Frame,
    area: Rect,
    tool: &str,
    subtitle: Option<&str>,
    tick: u64,
    theme: &Theme,
) {
    let p = &theme.palette;
    // A frame across the whole terminal: the splash owns the screen.
    let title = theme.app.brand.to_string();
    let right = subtitle
        .filter(|s| !s.is_empty())
        .map(|s| Line::from(Span::styled(s.to_string(), p.fg(p.dim))));
    let inner = crate::widgets::panel(f, area, &title, right, false, theme);

    // Scale the logotype to the height so a big terminal gets a big wordmark.
    let scale = if inner.height >= 30 {
        3
    } else if inner.height >= 16 {
        2
    } else {
        1
    };
    let mut lines: Vec<Line<'static>> = crate::widgets::logo(theme.app.brand, scale, theme)
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let color = if i == 0 { p.accent } else { p.accent2 };
            Line::from(Span::styled(row, p.bold(color)))
        })
        .collect();
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled(
            format!("{} ", theme.ui.spinner((tick % 256) as u8)),
            p.bold(p.accent),
        ),
        Span::styled(format!("loading {tool}{}", ellipsis(theme)), p.fg(p.fg)),
    ]));
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        theme.breadcrumb(&[theme.app.brand, tool]),
        p.fg(p.dim),
    )));

    let h = lines.len() as u16;
    let rect = centered(inner, inner.width, h.min(inner.height));
    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center).style(p.base()),
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

    fn text(theme: &Theme, tool: &str, subtitle: Option<&str>, tick: u64) -> String {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 20)).unwrap();
        term.draw(|f| {
            let area = f.area();
            render(f, area, tool, subtitle, tick, theme);
        })
        .unwrap();
        let buf = term.backend().buffer();
        (0..20)
            .flat_map(|y| (0..60).map(move |x| buf[(x, y)].symbol().to_string()))
            .collect()
    }

    #[test]
    fn splash_shows_spinner_loading_brand_and_breadcrumb() {
        let t = theme();
        let s = text(&t, "wayang-fw", Some("FIREWALL SYSTEM"), 0);
        assert!(s.contains("loading wayang-fw…"), "loading line: {s:?}");
        assert!(s.contains('⠋'), "spinner tick 0: {s:?}");
        assert!(s.contains("WAYANG FW"), "breadcrumb brand: {s:?}");
        assert!(s.contains("FIREWALL SYSTEM"), "subtitle: {s:?}");
    }

    #[test]
    fn splash_spinner_advances_with_tick_and_is_ascii_when_plain() {
        let t = theme();
        assert!(text(&t, "wayang-router", None, 1).contains('⠙'));
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
        let s = text(&plain, "wayang-fw", None, 1);
        assert!(s.contains("/ loading wayang-fw..."), "plain spinner: {s:?}");
        assert!(!s.contains('⠋'));
    }
}
