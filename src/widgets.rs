//! HUD building blocks: bracket panels, gauges, badges, keycaps and the
//! logotype. One implementation of the visual spec (`TUI-UX-REVAMP.md` §5).
//!
//! Every widget takes a [`Theme`] (product identity + palette + glyphs), so a
//! product's `App` is the only per-product wiring.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::focus::Focus;
use crate::theme::Theme;

// ─── logo ───────────────────────────────────────────────────────────────────

/// 4-pixel-high letters used by the logotype (drawn two pixel rows per text
/// row with half blocks). Covers every glyph in the wayang brands.
fn glyph(ch: char) -> [&'static str; 4] {
    match ch {
        'W' => ["X...X", "X.X.X", "X.X.X", ".X.X."],
        'A' => [".X.", "X.X", "XXX", "X.X"],
        'Y' => ["X.X", ".X.", ".X.", ".X."],
        'N' => ["X..X", "XX.X", "X.XX", "X..X"],
        'G' => ["XXX", "X..", "X.X", "XXX"],
        'F' => ["XXX", "X..", "XX.", "X.."],
        'R' => ["XX.", "X.X", "XX.", "X.X"],
        'O' => ["XXX", "X.X", "X.X", "XXX"],
        'U' => ["X.X", "X.X", "X.X", "XXX"],
        'T' => ["XXX", ".X.", ".X.", ".X."],
        'E' => ["XXX", "X..", "XX.", "XXX"],
        'S' => ["XXX", "X..", "..X", "XXX"],
        '-' => ["..", "..", "..", ".."],
        ' ' => [".", ".", ".", "."],
        _ => ["?", "?", "?", "?"],
    }
}

/// The pixel logotype for `word` at `scale`× (1 → 2 rows, 2 → 4 rows), or a
/// one-line ASCII title with `--plain`. `word` is the product brand
/// (`WAYANG FW` / `WAYANG ROUTER` / `WAYANG OS`).
pub fn logo(word: &str, scale: usize, theme: &Theme) -> Vec<String> {
    if theme.is_plain() {
        let spaced = word
            .split(' ')
            .map(|w| {
                w.chars()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("  ");
        return vec![format!("== {spaced} ==")];
    }

    let scale = scale.max(1);
    let mut px: Vec<Vec<bool>> = vec![Vec::new(); 4];
    for (i, ch) in word.chars().enumerate() {
        let g = glyph(ch);
        for (row, bits) in g.iter().enumerate() {
            if i > 0 {
                px[row].push(false);
            }
            px[row].extend(bits.chars().map(|c| c == 'X'));
        }
    }
    let rows: Vec<Vec<bool>> = px
        .iter()
        .flat_map(|r| {
            let wide: Vec<bool> = r.iter().flat_map(|&b| vec![b; scale]).collect();
            vec![wide; scale]
        })
        .collect();
    let n = rows.len() / 2;
    (0..n)
        .map(|i| {
            rows[2 * i]
                .iter()
                .zip(&rows[2 * i + 1])
                .map(|(&a, &b)| match (a, b) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (false, false) => ' ',
                })
                .collect()
        })
        .collect()
}

/// [`logo`] as styled lines: top half `accent`, bottom half `accent2`.
pub fn logo_lines(word: &str, theme: &Theme) -> Vec<Line<'static>> {
    let rows = logo(word, 1, theme);
    let n = rows.len();
    rows.into_iter()
        .enumerate()
        .map(|(i, row)| {
            let color = if i < n / 2 {
                theme.palette.accent
            } else {
                theme.palette.accent2
            };
            Line::from(Span::styled(row, theme.palette.bold(color)))
        })
        .collect()
}

// ─── panels ─────────────────────────────────────────────────────────────────

/// Draw a HUD panel (thin frame, heavy corners, `◢ TITLE ◣`) and return its
/// inner area. `right` is an optional right-aligned title.
///
/// This is the **unfocused** chrome. The pane that owns the keyboard must use
/// [`panel_focused`], so exactly one pane is unmistakable without colour
/// (spec §5, "Focused-pane highlight").
pub fn panel(
    f: &mut Frame,
    area: Rect,
    title: &str,
    right: Option<Line<'static>>,
    focused: bool,
    theme: &Theme,
) -> Rect {
    draw_panel(f, area, title, right, Focus::from(focused), theme)
}

/// The **focused** panel: `accent` frame + title and a `▸ ` title prefix
/// (`> ` on `--plain`/mono). In mono the prefix is the whole signal, so focus
/// never depends on colour.
pub fn panel_focused(
    f: &mut Frame,
    area: Rect,
    title: &str,
    right: Option<Line<'static>>,
    theme: &Theme,
) -> Rect {
    draw_panel(f, area, title, right, Focus::Focused, theme)
}

fn draw_panel(
    f: &mut Frame,
    area: Rect,
    title: &str,
    right: Option<Line<'static>>,
    focus: Focus,
    theme: &Theme,
) -> Rect {
    let p = &theme.palette;
    let ui = theme.ui;
    let focused = focus.is_focused();
    let frame = if focused { p.accent } else { p.border };
    let title_style = focus.title_style(theme);
    let chevron = if focused { p.accent2 } else { p.border };
    let title_line = if ui.plain {
        Line::from(Span::styled(
            format!("[ {}{title} ]", focus.prefix(theme)),
            title_style,
        ))
    } else {
        let mut spans = vec![Span::styled("◢ ", p.fg(chevron))];
        if focused {
            spans.push(Span::styled(
                format!("{} ", theme.focus_mark()),
                p.bold(p.accent),
            ));
        }
        spans.push(Span::styled(title.to_string(), title_style));
        spans.push(Span::styled(" ◣", p.fg(chevron)));
        Line::from(spans)
    };
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_set(ui.border())
        .border_style(p.fg(frame))
        .style(p.base())
        .title(title_line);
    if let Some(r) = right {
        block = block.title_top(r.right_aligned());
    }
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some([tl, tr, bl, br]) = ui.corners() {
        if area.width >= 2 && area.height >= 2 {
            let (x0, y0) = (area.x, area.y);
            let (x1, y1) = (area.right() - 1, area.bottom() - 1);
            let style = p.fg(frame);
            let buf = f.buffer_mut();
            for (x, y, sym) in [(x0, y0, tl), (x1, y0, tr), (x0, y1, bl), (x1, y1, br)] {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(sym).set_style(style);
                }
            }
        }
    }
    inner
}

// ─── header / footer ────────────────────────────────────────────────────────

/// What a header shows: brand · breadcrumb · glyph+word badge · revision,
/// plus optional product-specific right-hand spans.
#[derive(Debug, Clone, Default)]
pub struct Header<'a> {
    /// `MODULE ▸ TAB ▸ item`; empty parts are dropped.
    pub breadcrumb: Vec<&'a str>,
    /// State badge as `(severity, word)` — always glyph **and** word.
    pub badge: Option<(u8, &'a str)>,
    /// Revision label; `"demo"` renders as a label, not `vdemo`.
    pub revision: Option<&'a str>,
    /// Extra left-hand context, e.g. `FIREWALL SYSTEM`.
    pub subtitle: Option<&'a str>,
    /// Extra right-hand spans (host, `DEMO DATA`, …), before the badge.
    pub right: Vec<Span<'static>>,
}

impl<'a> Header<'a> {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Draw the one-row header bar: `bar` background, brand on the left,
/// breadcrumb, then right-aligned extras + badge + revision. Parts that don't
/// fit are dropped, breadcrumb and subtitle first.
pub fn header(f: &mut Frame, area: Rect, h: &Header, theme: &Theme) {
    let p = &theme.palette;
    let ui = theme.ui;
    let bar = match p.bar {
        Some(bg) => p.base().bg(bg),
        None => p.base(),
    };
    f.render_widget(Block::default().style(bar), area);

    let mut left = vec![
        if ui.plain {
            Span::raw(" ")
        } else {
            Span::styled(" ◢◤ ", p.fg(p.accent2))
        },
        Span::styled(theme.app.brand.to_string(), p.bold(p.accent)),
    ];
    let mut right: Vec<Span> = h.right.clone();
    if let Some((sev, word)) = h.badge {
        right.push(badge(sev, word, theme));
    }

    let width = area.width as usize;
    let used = |spans: &[Span]| spans.iter().map(|s| s.width()).sum::<usize>();
    let fits =
        |left: &[Span], right: &[Span], extra: usize| used(left) + used(right) + extra < width;

    if let Some(rev) = h.revision.filter(|r| !r.is_empty()) {
        let text = if rev == "demo" {
            "  demo".to_string()
        } else {
            format!("  v{rev}")
        };
        let span = Span::styled(text, p.fg(p.dim));
        if fits(&left, &right, span.width()) {
            right.push(span);
        }
    }
    if let Some(sub) = h.subtitle {
        let span = Span::styled(format!(" // {sub}"), p.fg(p.dim));
        if fits(&left, &right, span.width()) {
            left.push(span);
        }
    }
    if h.breadcrumb.iter().any(|s| !s.is_empty()) {
        let span = Span::styled(
            format!(" {} {} ", ui.arrow(), theme.breadcrumb(&h.breadcrumb)),
            p.fg(p.accent2),
        );
        if fits(&left, &right, span.width()) {
            left.push(span);
        }
    }

    let right = Line::from(right);
    let rw = (right.width() as u16).min(area.width);
    let [l, r] = Layout::horizontal([Constraint::Min(1), Constraint::Length(rw)]).areas(area);
    f.render_widget(Paragraph::new(Line::from(left)).style(bar), l);
    f.render_widget(
        Paragraph::new(right).alignment(Alignment::Right).style(bar),
        r,
    );
}

/// Draw the one-row footer: context keycaps on the left, an optional status
/// span (glyph + word) right-aligned.
pub fn footer(
    f: &mut Frame,
    area: Rect,
    keys: &[(&str, &str)],
    status: Option<Span<'static>>,
    theme: &Theme,
) {
    let p = &theme.palette;
    let bar = match p.bar {
        Some(bg) => p.base().bg(bg),
        None => p.base(),
    };
    f.render_widget(Block::default().style(bar), area);

    let mut right: Vec<Span> = Vec::new();
    if let Some(s) = status {
        right.push(Span::raw(" "));
        right.push(s);
        right.push(Span::raw(" "));
    }
    let rw = (right.iter().map(|s| s.width()).sum::<usize>() as u16).min(area.width);
    let [l, r] = Layout::horizontal([Constraint::Min(1), Constraint::Length(rw)]).areas(area);
    f.render_widget(Paragraph::new(keycaps(keys, theme)).style(bar), l);
    f.render_widget(
        Paragraph::new(Line::from(right))
            .alignment(Alignment::Right)
            .style(bar),
        r,
    );
}

// ─── rows / chips ───────────────────────────────────────────────────────────

/// A list row: selected rows get the `selection` highlight and a cursor
/// marker, so mono keeps a visible `>` even though the highlight is reversed.
pub fn selection_row(content: Vec<Span<'static>>, selected: bool, theme: &Theme) -> Line<'static> {
    if !selected {
        return Line::from(content);
    }
    let style = theme.palette.highlight();
    let mut spans = vec![Span::styled(theme.selection_mark(), style)];
    spans.extend(
        content
            .into_iter()
            .map(|s| Span::styled(s.content.into_owned(), style)),
    );
    Line::from(spans)
}

/// Coloured status chip, e.g. ` ✔ OK ` on green. Mono renders `[✔ OK]`.
pub fn badge(sev: u8, label: &str, theme: &Theme) -> Span<'static> {
    let p = &theme.palette;
    if p.is_mono() {
        return Span::styled(
            format!("[{} {label}]", theme.ui.sym(sev)),
            Style::default().add_modifier(Modifier::BOLD),
        );
    }
    Span::styled(
        format!(" {} {label} ", theme.ui.sym(sev)),
        Style::default()
            .bg(p.severity(sev))
            .fg(p.on_badge)
            .add_modifier(Modifier::BOLD),
    )
}

/// Status text without a background, e.g. `✔ OK` in green.
pub fn status(sev: u8, label: &str, theme: &Theme) -> Span<'static> {
    let p = &theme.palette;
    Span::styled(
        format!("{} {label}", theme.ui.sym(sev)),
        p.bold(p.severity(sev)),
    )
}

/// Section caption inside a panel, e.g. `── ALERTS ─────`.
pub fn caption(text: &str, width: u16, theme: &Theme) -> Line<'static> {
    let p = &theme.palette;
    let ui = theme.ui;
    let lead = if ui.plain { "-- " } else { "── " };
    let fill = if ui.plain { "-" } else { "─" };
    let used = text.chars().count() + 4;
    let rest = (width as usize).saturating_sub(used);
    Line::from(vec![
        Span::styled(lead, p.fg(p.border)),
        Span::styled(text.to_string(), p.bold(p.accent)),
        Span::styled(format!(" {}", fill.repeat(rest)), p.fg(p.border)),
    ])
}

// ─── fields / gauges ────────────────────────────────────────────────────────

/// Footer key hints: ` key ` keycaps followed by dim labels. Without a `bar`
/// background (ansi/mono) the keycap is `[key]`.
pub fn keycaps(items: &[(&str, &str)], theme: &Theme) -> Line<'static> {
    let p = &theme.palette;
    let mut spans = Vec::new();
    for (key, label) in items {
        match p.bar {
            Some(bg) => spans.push(Span::styled(
                format!(" {key} "),
                Style::default()
                    .bg(bg)
                    .fg(p.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            None => spans.push(Span::styled(format!("[{key}]"), p.bold(p.accent))),
        }
        spans.push(Span::styled(format!(" {label}  "), p.fg(p.dim)));
    }
    Line::from(spans)
}

/// A labelled line gauge: `LABEL     ━━━━━━━──── 62%`.
#[allow(clippy::too_many_arguments)]
pub fn gauge(
    label: &str,
    label_w: usize,
    percent: f64,
    cells: usize,
    color: Color,
    value: &str,
    theme: &Theme,
) -> Line<'static> {
    let p = &theme.palette;
    let (full, empty) = theme.ui.gauge_cells(p.is_mono());
    let pct = if percent.is_finite() {
        percent.clamp(0.0, 100.0)
    } else {
        0.0
    };
    let filled = (((pct / 100.0) * cells as f64).round() as usize).min(cells);
    Line::from(vec![
        Span::styled(format!("{label:<label_w$}"), p.fg(p.dim)),
        Span::styled(full.repeat(filled), p.bold(color)),
        Span::styled(empty.repeat(cells - filled), p.fg(p.track)),
        Span::styled(format!(" {value}"), p.bold(p.fg)),
    ])
}

/// Gauge cell count that fits `width` next to a label and a value.
pub fn gauge_cells_for(width: u16, label_w: usize, value_w: usize) -> usize {
    (width as usize)
        .saturating_sub(label_w + value_w + 1)
        .clamp(4, 32)
}

/// `LABEL     value` row.
pub fn field(
    label: &str,
    label_w: usize,
    value: Vec<Span<'static>>,
    theme: &Theme,
) -> Line<'static> {
    let p = &theme.palette;
    let mut spans = vec![Span::styled(format!("{label:<label_w$}"), p.fg(p.dim))];
    spans.extend(value);
    Line::from(spans)
}

/// Truncate to `max` display chars with an ellipsis (`~` when plain).
pub fn clip(s: &str, max: usize, theme: &Theme) -> String {
    theme.clip(s, max)
}

/// Centered sub-rectangle of at most `w` x `h`.
pub fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ColorMode, Flags, Palette, WAYANG_FW};

    fn theme(flags: Flags, no_color: bool, color: Option<&str>) -> Theme {
        Theme::from_env(WAYANG_FW, flags, no_color, color, Some("24bit"))
    }

    fn ansi() -> Theme {
        Theme::from_env(WAYANG_FW, Flags::default(), false, Some("ansi"), None)
    }

    fn mono() -> Theme {
        Theme::from_env(WAYANG_FW, Flags::default(), true, None, None)
    }

    #[test]
    fn logo_has_two_rows_and_blocks() {
        let t = theme(Flags::default(), false, Some("truecolor"));
        let rows = logo("WAYANG FW", 1, &t);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].contains('█') || rows[0].contains('▀'), "{rows:?}");
        let lines = logo_lines("WAYANG ROUTER", &t);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn plain_logo_is_ascii_and_branded() {
        let t = theme(
            Flags {
                plain: true,
                ..Flags::default()
            },
            false,
            Some("truecolor"),
        );
        assert_eq!(logo("WAYANG OS", 1, &t), vec!["== W A Y A N G  O S =="]);
    }

    #[test]
    fn caption_fills_to_the_width_in_border_colour() {
        let line = caption("ACTIONS", 24, &ansi()).to_string();
        assert_eq!(line, "── ACTIONS ─────────────");
        assert_eq!(line.chars().count(), 24);
        // Plain uses ASCII.
        let plain = theme(
            Flags {
                plain: true,
                ..Flags::default()
            },
            false,
            Some("ansi"),
        );
        assert!(caption("ACTIONS", 20, &plain)
            .to_string()
            .starts_with("-- ACTIONS -"));
    }

    #[test]
    fn keycap_shapes_per_mode() {
        // neon: ` key ` on the bar background.
        let neon = theme(Flags::default(), false, Some("truecolor"));
        let s = keycaps(&[("q", "quit")], &neon).to_string();
        assert_eq!(s, " q  quit  ");
        // ansi has no bar → bracketed key.
        assert_eq!(keycaps(&[("q", "quit")], &ansi()).to_string(), "[q] quit  ");
    }

    #[test]
    fn mono_badge_is_bracketed_glyph_and_word() {
        let s = badge(0, "OK", &mono()).to_string();
        assert_eq!(s, "[✔ OK]");
        // The coloured badge is padded and carries the severity colour.
        let span = badge(0, "OK", &theme(Flags::default(), false, Some("truecolor")));
        assert_eq!(span.content, " ✔ OK ");
        assert_eq!(span.style.bg, Some(Color::Rgb(57, 255, 136)));
    }

    #[test]
    fn status_has_no_background() {
        let span = status(
            4,
            "UNREACHABLE",
            &theme(Flags::default(), false, Some("truecolor")),
        );
        assert_eq!(span.content, "✖ UNREACHABLE");
        assert_eq!(span.style.bg, None);
    }

    #[test]
    fn selection_row_marks_and_highlights() {
        let t = ansi();
        let sel = selection_row(vec![Span::raw("eth0")], true, &t).to_string();
        assert!(sel.contains("eth0"));
        assert!(sel.contains("▶ "), "fancy cursor: {sel:?}");
        let mono_row = selection_row(vec![Span::raw("eth0")], true, &mono()).to_string();
        assert!(mono_row.contains("> eth0"), "{mono_row:?}");
        let plain = selection_row(vec![Span::raw("eth0")], false, &t).to_string();
        assert_eq!(plain, "eth0");
    }

    #[test]
    fn clip_and_centered() {
        let t = ansi();
        assert_eq!(clip("hello world", 8, &t), "hello w…");
        assert_eq!(clip("short", 8, &t), "short");
        assert_eq!(clip("abc", 0, &t), "");
        let area = Rect::new(0, 0, 80, 24);
        let c = centered(area, 40, 10);
        assert_eq!((c.x, c.y, c.width, c.height), (20, 7, 40, 10));
    }

    #[test]
    fn gauge_clamps_and_counts_cells() {
        let t = ansi();
        let line = gauge("CPU", 6, 62.0, 10, t.palette.ok, "62%", &t).to_string();
        assert!(line.starts_with("CPU   "), "{line:?}");
        assert!(line.ends_with("62%"), "{line:?}");
        // Over 100% is clamped, never panics or overshoots.
        let over = gauge("X", 2, 250.0, 8, t.palette.bad, "250%", &t).to_string();
        assert!(over.contains("250%"));
        assert_eq!(gauge_cells_for(80, 12, 6), 32); // clamped to 32
    }

    fn panel_text(theme: &Theme, focused: bool) -> String {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(30, 3)).unwrap();
        term.draw(|f| {
            let area = f.area();
            panel(f, area, "TARGET", None, focused, theme);
        })
        .unwrap();
        let buf = term.backend().buffer();
        (0..3)
            .flat_map(|y| (0..30).map(move |x| buf[(x, y)].symbol().to_string()))
            .collect()
    }

    #[test]
    fn focus_is_a_glyph_not_colour() {
        let fancy = ansi();
        assert!(panel_text(&fancy, true).contains("▸ TARGET"));
        assert!(!panel_text(&fancy, false).contains('▸'));
        // NO_COLOR: the marker is a plain `>`, so focus survives with no colour.
        let m = mono();
        assert!(panel_text(&m, true).contains("> TARGET"));
        assert!(!panel_text(&m, false).contains('>'));
        assert_ne!(panel_text(&m, true), panel_text(&m, false));
    }

    #[test]
    fn palette_is_the_spec_palette() {
        let p = Palette::new(ColorMode::Neon, false, false);
        assert_eq!(p.accent, Color::Rgb(0, 229, 255));
    }
}
