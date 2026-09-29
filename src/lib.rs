//! # wayang-tui
//!
//! Shared ratatui **component library** for the wayang HUDs — the `wayang`
//! CLI, `wayang-fw` and `wayang-router`. One implementation of the look and
//! feel, so the three are visually identical *by construction* instead of by
//! copy-pasted `theme.rs` / `widgets.rs`.
//!
//! Canonical visual spec: `wayangos/docs/TUI-UX-REVAMP.md` (§5, "Visual spec"
//! + "Focused-pane highlight").
//!
//! ## Status
//!
//! Scaffold. Components are extracted from `wayang-fw`/`wayang-router` once
//! their UX wave lands. Planned modules:
//!
//! * [`theme`]  — `Palette`, `Ui` (neon / ansi / mono / light, `NO_COLOR`,
//!   `--plain`, width thresholds).
//! * [`widgets`] — `panel(title, focused, right)`, `caption`, `keycaps`,
//!   `header`, `footer`, `selection_row`, `badge`, `status`, `gauge`, `field`,
//!   `logo`, `clip`, `centered`.
//! * `focus`   — the active-pane model and the colour-free `▸`/dim rule.
//! * `overlay` — the help / review / quick-jump frames shared by all three.

/// Theme: palette + rendering mode (neon / ansi / mono / light).
pub mod theme {}

/// Widgets: the bracket panels and building blocks.
pub mod widgets {}
