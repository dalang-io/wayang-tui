//! # wayang-tui
//!
//! Shared ratatui **component library** for the wayang HUDs — the `wayang`
//! CLI, `wayang-fw` and `wayang-router`. One implementation of the look and
//! feel, so the products are visually identical *by construction* instead of
//! by copy-pasted `theme.rs` / `widgets.rs`.
//!
//! Canonical visual spec: `wayangos/docs/TUI-UX-REVAMP.md` §5 ("Visual spec" +
//! "Focused-pane highlight").
//!
//! ## Modules
//!
//! * [`theme`] — [`Theme::resolve`](theme::Theme::resolve): the neon/ansi/mono
//!   palette (exact spec RGBs), the glyph set, and the per-product
//!   [`App`](theme::App) identity (`WAYANG_FW_COLOR`, logo word, …).
//! * [`widgets`] — [`panel`](widgets::panel) /
//!   [`panel_focused`](widgets::panel_focused), [`caption`](widgets::caption),
//!   [`keycaps`](widgets::keycaps), [`header`](widgets::header),
//!   [`footer`](widgets::footer), [`selection_row`](widgets::selection_row),
//!   [`badge`](widgets::badge), [`status`](widgets::status),
//!   [`gauge`](widgets::gauge), [`field`](widgets::field),
//!   [`logo`](widgets::logo), [`clip`](widgets::clip),
//!   [`centered`](widgets::centered).
//! * [`focus`] — the focused-pane model ([`Focus`](focus::Focus),
//!   [`FocusRing`](focus::FocusRing)) and the colour-free `▸`/`>` marker rule.
//! * [`overlay`] — the shared help / REVIEW / quick-jump frame.
//!
//! ## Wiring a product
//!
//! ```no_run
//! use wayang_tui::theme::{Flags, Theme, WAYANG_FW};
//!
//! let theme = Theme::resolve(WAYANG_FW, Flags::default());
//! ```
//!
//! The product parses `--plain` / `--light` / `--mono` / `--transparent` into
//! [`Flags`](theme::Flags); everything else comes from the environment (see
//! the [`theme`] module docs for precedence).

pub mod focus;
pub mod overlay;
pub mod theme;
pub mod widgets;
