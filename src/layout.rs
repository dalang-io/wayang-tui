//! The §5d layout standard: **one tab = one full-screen view.**
//!
//! Spec: `wayangos/docs/TUI-UX-REVAMP.md` §5d. Today the products arrange
//! panes ad-hoc per screen (side-by-side here, stacked there, full-screen
//! elsewhere), so switching tabs makes the user re-learn where the content
//! lives. The rule that replaces it:
//!
//! 1. a tab's body is **full-screen** — the tab row and breadcrumb are the
//!    only navigation chrome, and there are no ad-hoc side-by-side/stacked
//!    panes;
//! 2. where a master–detail is genuinely useful, it is the **same fixed
//!    bottom DETAIL strip** on every screen (list on top), never left/right
//!    here and top/bottom there;
//! 3. with a full-screen tab the focused thing is the list (or the open
//!    modal), so "which pane owns the keys" stops being ambiguous.
//!
//! [`body`] returns that split.
//!
//! ```no_run
//! # use ratatui::Frame;
//! # fn demo(f: &mut Frame, t: &wayang_tui::theme::Theme) {
//! use wayang_tui::layout;
//!
//! // header drawn separately; then:
//! let areas = layout::body(f.area(), true);
//! // widgets::tab_row(f, areas.tab_row, …);
//! // … render the full-screen tab into areas.content …
//! // widgets::panel(f, areas.detail.unwrap(), "DETAIL", …);
//! # let _ = (&areas.content, t);
//! # }
//! ```

use ratatui::layout::{Constraint, Layout as RatatuiLayout, Rect};

/// Height of the tab-row strip (`STATIC │ KERNEL │ BGP`).
pub const TAB_ROW_HEIGHT: u16 = 1;
/// Default height of the fixed bottom DETAIL strip.
pub const DETAIL_HEIGHT: u16 = 6;

/// Body regions for one full-screen tab.
///
/// All rectangles are relative to the `area` passed to [`body`]; the detail
/// strip is `None` when the tab has no detail treatment (or there is not room
/// for one plus at least one content row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// The tab-row strip at the top, [`TAB_ROW_HEIGHT`] rows.
    pub tab_row: Rect,
    /// The full-screen tab content, between the tab row and the detail strip.
    pub content: Rect,
    /// The fixed bottom DETAIL strip, when present.
    pub detail: Option<Rect>,
}

/// Split `area` into the §5d body (`tab_row`, `content`, optional `detail`).
///
/// Uses [`DETAIL_HEIGHT`] for the strip; see [`body_sized`] to override. When
/// `has_detail` is set but `area` is too short to keep at least one content
/// row above the strip, the detail is dropped and `content` takes the space —
/// a tiny screen degrades to full-screen rather than losing the list.
pub fn body(area: Rect, has_detail: bool) -> Layout {
    body_sized(area, has_detail, DETAIL_HEIGHT)
}

/// [`body`] with an explicit detail-strip height.
pub fn body_sized(area: Rect, has_detail: bool, detail_height: u16) -> Layout {
    let tab_h = TAB_ROW_HEIGHT.min(area.height);
    let rest = area.height - tab_h;
    let detail_h = if has_detail && detail_height > 0 && rest > detail_height {
        detail_height
    } else {
        0
    };
    let content_h = rest - detail_h;

    let rects = RatatuiLayout::vertical([
        Constraint::Length(tab_h),
        Constraint::Length(content_h),
        Constraint::Length(detail_h),
    ])
    .split(area);

    Layout {
        tab_row: rects[0],
        content: rects[1],
        detail: (detail_h > 0).then_some(rects[2]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_screen_tab_has_a_tab_row_and_no_detail() {
        let area = Rect::new(0, 0, 80, 24);
        let l = body(area, false);
        assert_eq!(l.tab_row, Rect::new(0, 0, 80, 1));
        assert_eq!(l.content, Rect::new(0, 1, 80, 23));
        assert_eq!(l.detail, None);
    }

    #[test]
    fn detail_is_a_fixed_bottom_strip() {
        let area = Rect::new(0, 0, 80, 24);
        let l = body(area, true);
        assert_eq!(l.tab_row, Rect::new(0, 0, 80, 1));
        assert_eq!(l.content, Rect::new(0, 1, 80, 17));
        assert_eq!(l.detail, Some(Rect::new(0, 18, 80, DETAIL_HEIGHT)));
        // The three regions tile the area without overlap.
        assert_eq!(
            l.tab_row.height + l.content.height + l.detail.unwrap().height,
            area.height
        );
    }

    #[test]
    fn short_screen_degrades_to_full_screen() {
        let area = Rect::new(2, 5, 40, 6);
        let l = body(area, true);
        assert_eq!(l.tab_row, Rect::new(2, 5, 40, 1));
        assert_eq!(l.content, Rect::new(2, 6, 40, 5), "keeps the whole rest");
        assert_eq!(l.detail, None, "no room for a strip and a content row");
    }

    #[test]
    fn explicit_detail_height_is_honoured() {
        let area = Rect::new(0, 0, 60, 20);
        let l = body_sized(area, true, 3);
        assert_eq!(l.content, Rect::new(0, 1, 60, 16));
        assert_eq!(l.detail, Some(Rect::new(0, 17, 60, 3)));
    }

    #[test]
    fn zero_height_area_is_safe() {
        let l = body(Rect::new(0, 0, 10, 0), true);
        assert_eq!(l.content.height, 0);
        assert_eq!(l.detail, None);
    }
}
