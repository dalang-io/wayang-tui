//! Palette, glyph set and product identity for the wayang HUDs.
//!
//! One implementation of the canonical visual spec
//! (`wayangos/docs/TUI-UX-REVAMP.md` §5, "Visual spec"), parameterised by the
//! product so `wayang-fw`, `wayang-router` and the `wayang` CLI render through
//! the *same* code:
//!
//! * [`App`] — the per-product identity: the env-var prefix
//!   (`WAYANG_FW_COLOR` vs `WAYANG_ROUTER_COLOR`), the tool name and the logo
//!   word (`WAYANG FW` / `WAYANG ROUTER` / `WAYANG OS`).
//! * [`ColorMode`] — **neon** (24-bit RGB), **ansi** (the terminal's own 16
//!   colours) or **mono** (`NO_COLOR`, no colour at all).
//! * [`Palette`] — the resolved colours. The neon_dark RGB values are fixed by
//!   the spec and asserted in tests.
//! * [`Ui`] — the glyph set: Unicode HUD symbols, or ASCII with `--plain`.
//! * [`Theme`] — `App` + `Palette` + `Ui`, what every widget takes.
//!
//! # Resolution
//!
//! [`Theme::resolve`] is the entry point: it applies CLI [`Flags`] on top of
//! the environment, in this order:
//!
//! 1. `--mono` (explicit CLI request) → [`ColorMode::Mono`];
//! 2. `NO_COLOR` → [`ColorMode::Mono`];
//! 3. `<PREFIX>_COLOR` (e.g. `WAYANG_FW_COLOR`) → `truecolor|ansi|none`;
//! 4. the crate-wide `WAYANG_TUI_COLOR` override;
//! 5. `COLORTERM=truecolor|24bit` → [`ColorMode::Neon`], else
//!    [`ColorMode::Ansi`].
//!
//! `--light` picks the light palette, `--plain` swaps every Unicode symbol for
//! ASCII and `--transparent` drops the forced page background. [`Theme::from_env`]
//! does the same without touching the process environment, for tests.

use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;

/// Per-product identity shared by every widget.
///
/// `env_prefix` names the colour override (`WAYANG_FW` →
/// `WAYANG_FW_COLOR`); `brand` is the word the [logo](crate::widgets::logo)
/// draws (`WAYANG FW`, `WAYANG ROUTER`, `WAYANG OS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct App {
    /// Env-var prefix, without the `_COLOR` suffix (e.g. `"WAYANG_FW"`).
    pub env_prefix: &'static str,
    /// Binary name, for messages (e.g. `"wayang-fw"`).
    pub tool: &'static str,
    /// Logo word, spaced as it should read (e.g. `"WAYANG ROUTER"`).
    pub brand: &'static str,
}

/// `wayang-fw` identity (`WAYANG_FW_COLOR`, logo `WAYANG FW`).
pub const WAYANG_FW: App = App {
    env_prefix: "WAYANG_FW",
    tool: "wayang-fw",
    brand: "WAYANG FW",
};

/// `wayang-router` identity (`WAYANG_ROUTER_COLOR`, logo `WAYANG ROUTER`).
pub const WAYANG_ROUTER: App = App {
    env_prefix: "WAYANG_ROUTER",
    tool: "wayang-router",
    brand: "WAYANG ROUTER",
};

/// The `wayang` CLI / `wayang-os` identity (logo `WAYANG OS`).
pub const WAYANG_OS: App = App {
    env_prefix: "WAYANG_OS",
    tool: "wayang",
    brand: "WAYANG OS",
};

/// CLI display flags, product-independent. All default to `false`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    /// `--plain`: ASCII glyphs instead of Unicode.
    pub plain: bool,
    /// `--light`: the light palette (neon_light).
    pub light: bool,
    /// `--mono`: force [`ColorMode::Mono`] regardless of the environment.
    pub mono: bool,
    /// `--transparent`: keep the terminal's own background.
    pub transparent: bool,
}

/// Colour fidelity of the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// 24-bit RGB (the canonical neon palette).
    Neon,
    /// The terminal's own 16-colour palette.
    Ansi,
    /// No colour at all; selection/focus is marked with glyphs + bold.
    Mono,
}

impl ColorMode {
    /// Resolve from the process environment for `app`, honouring `flags`.
    pub fn detect(app: &App, flags: Flags) -> Self {
        let color = own_color(app);
        Self::from_env(
            flags,
            std::env::var_os("NO_COLOR").is_some(),
            color.as_deref(),
            std::env::var("COLORTERM").ok().as_deref(),
        )
    }

    /// Pure resolution — no environment access, so it is testable.
    ///
    /// `no_color` is `NO_COLOR`, `color` is the value of `<PREFIX>_COLOR` (or
    /// `WAYANG_TUI_COLOR`), `colorterm` is `COLORTERM`.
    pub fn from_env(
        flags: Flags,
        no_color: bool,
        color: Option<&str>,
        colorterm: Option<&str>,
    ) -> Self {
        if flags.mono || no_color {
            return ColorMode::Mono;
        }
        match color.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            Some("truecolor" | "24bit" | "rgb" | "neon") => return ColorMode::Neon,
            Some("ansi" | "16" | "basic") => return ColorMode::Ansi,
            Some("none" | "mono" | "off") => return ColorMode::Mono,
            _ => {}
        }
        match colorterm.map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("truecolor" | "24bit") => ColorMode::Neon,
            _ => ColorMode::Ansi,
        }
    }
}

/// The product's colour override: `<PREFIX>_COLOR`, then the crate-wide
/// `WAYANG_TUI_COLOR`.
fn own_color(app: &App) -> Option<String> {
    let own = format!("{}_COLOR", app.env_prefix);
    std::env::var(own)
        .or_else(|_| std::env::var("WAYANG_TUI_COLOR"))
        .ok()
}

/// Resolved colours. The neon_dark values are fixed by the spec.
#[derive(Debug, Clone)]
pub struct Palette {
    pub mode: ColorMode,
    /// Primary accent (titles, logo, focus).
    pub accent: Color,
    /// Secondary accent (logo glyphs, selection).
    pub accent2: Color,
    /// Labels and secondary text.
    pub dim: Color,
    /// Panel borders.
    pub border: Color,
    /// Empty part of a gauge.
    pub track: Color,
    pub ok: Color,
    pub warn: Color,
    pub bad: Color,
    pub fg: Color,
    /// Forced page background; `None` keeps the terminal's.
    pub bg: Option<Color>,
    /// Header bar / keycap background.
    pub bar: Option<Color>,
    /// Text drawn on top of a coloured badge.
    pub on_badge: Color,
    selection: Option<(Color, Color)>,
}

impl Palette {
    /// Build the palette for `mode`; `light` selects the light variant,
    /// `transparent` drops the forced background.
    pub fn new(mode: ColorMode, light: bool, transparent: bool) -> Self {
        let mut p = match (mode, light) {
            (ColorMode::Mono, _) => Palette::mono(),
            (ColorMode::Neon, false) => Palette::neon_dark(),
            (ColorMode::Neon, true) => Palette::neon_light(),
            (ColorMode::Ansi, light) => Palette::ansi(light),
        };
        if transparent {
            p.bg = None;
        }
        p
    }

    fn neon_dark() -> Self {
        Palette {
            mode: ColorMode::Neon,
            accent: Color::Rgb(0, 229, 255),
            accent2: Color::Rgb(255, 46, 151),
            dim: Color::Rgb(112, 132, 152),
            border: Color::Rgb(28, 74, 92),
            track: Color::Rgb(34, 46, 60),
            ok: Color::Rgb(57, 255, 136),
            warn: Color::Rgb(255, 176, 0),
            bad: Color::Rgb(255, 59, 59),
            fg: Color::Rgb(196, 210, 224),
            bg: Some(Color::Rgb(10, 14, 20)),
            bar: Some(Color::Rgb(18, 26, 38)),
            on_badge: Color::Rgb(10, 14, 20),
            selection: Some((Color::Rgb(74, 14, 52), Color::Rgb(255, 255, 255))),
        }
    }

    fn neon_light() -> Self {
        Palette {
            mode: ColorMode::Neon,
            accent: Color::Rgb(0, 112, 160),
            accent2: Color::Rgb(196, 0, 112),
            dim: Color::Rgb(96, 110, 126),
            border: Color::Rgb(150, 182, 200),
            track: Color::Rgb(214, 222, 230),
            ok: Color::Rgb(0, 140, 72),
            warn: Color::Rgb(176, 104, 0),
            bad: Color::Rgb(204, 24, 24),
            fg: Color::Rgb(20, 30, 40),
            bg: Some(Color::Rgb(242, 246, 250)),
            bar: Some(Color::Rgb(222, 232, 240)),
            on_badge: Color::Rgb(255, 255, 255),
            selection: Some((Color::Rgb(255, 214, 236), Color::Rgb(20, 30, 40))),
        }
    }

    fn ansi(light: bool) -> Self {
        Palette {
            mode: ColorMode::Ansi,
            accent: if light { Color::Blue } else { Color::Cyan },
            accent2: Color::Magenta,
            dim: if light { Color::DarkGray } else { Color::Gray },
            border: if light { Color::Blue } else { Color::DarkGray },
            track: Color::DarkGray,
            ok: Color::Green,
            warn: Color::Yellow,
            bad: Color::Red,
            fg: if light { Color::Black } else { Color::Gray },
            bg: Some(if light { Color::White } else { Color::Black }),
            bar: None,
            on_badge: Color::Black,
            selection: None,
        }
    }

    fn mono() -> Self {
        Palette {
            mode: ColorMode::Mono,
            accent: Color::Reset,
            accent2: Color::Reset,
            dim: Color::Reset,
            border: Color::Reset,
            track: Color::Reset,
            ok: Color::Reset,
            warn: Color::Reset,
            bad: Color::Reset,
            fg: Color::Reset,
            bg: None,
            bar: None,
            on_badge: Color::Reset,
            selection: None,
        }
    }

    pub fn is_mono(&self) -> bool {
        self.mode == ColorMode::Mono
    }

    /// Default text style on the page background.
    pub fn base(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg.unwrap_or(Color::Reset))
    }

    pub fn fg(&self, color: Color) -> Style {
        Style::default().fg(color)
    }

    pub fn bold(&self, color: Color) -> Style {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    /// Selected row / menu entry.
    pub fn highlight(&self) -> Style {
        match self.selection {
            Some((bg, fg)) => Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD),
            // Reverse video stays readable on any terminal theme.
            None => Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
        }
    }

    /// Colour for a severity: 0 ok, 1 unknown, 2 warn, 3 backup, 4 replace.
    pub fn severity(&self, sev: u8) -> Color {
        match sev {
            0 => self.ok,
            2 => self.warn,
            3 | 4 => self.bad,
            _ => self.dim,
        }
    }

    /// Colour for a reading where higher is worse.
    pub fn level(&self, value: f64, warn_at: f64, bad_at: f64) -> Color {
        if value >= bad_at {
            self.bad
        } else if value >= warn_at {
            self.warn
        } else {
            self.ok
        }
    }
}

/// Glyph set: Unicode HUD symbols, or ASCII with `--plain`.
///
/// Colour is orthogonal — a terminal in mono mode still got here with
/// `plain: false`, and mono-aware markers live on [`Theme`].
#[derive(Debug, Clone, Copy)]
pub struct Ui {
    pub plain: bool,
}

const HUD_BORDER: border::Set = border::Set {
    top_left: "┌",
    top_right: "┐",
    bottom_left: "└",
    bottom_right: "┘",
    vertical_left: "│",
    vertical_right: "│",
    horizontal_top: "─",
    horizontal_bottom: "─",
};

const ASCII_BORDER: border::Set = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

/// Running on the Linux kernel console (TERM=linux, e.g. WayangOS's tty).
fn kernel_console() -> bool {
    static CONSOLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CONSOLE.get_or_init(|| std::env::var("TERM").is_ok_and(|t| t == "linux"))
}

const SPIN_FANCY: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPIN_PLAIN: [&str; 4] = ["|", "/", "-", "\\"];

impl Ui {
    pub const fn new(plain: bool) -> Self {
        Ui { plain }
    }

    pub fn border(self) -> border::Set {
        if self.plain {
            ASCII_BORDER
        } else {
            HUD_BORDER
        }
    }

    /// Heavy corner brackets drawn over the panel corners (HUD look).
    pub fn corners(self) -> Option<[&'static str; 4]> {
        (!self.plain).then_some(["┏", "┓", "┗", "┛"])
    }

    /// Selection cursor glyph (`▶` fancy, `>` plain).
    pub fn cursor(self) -> &'static str {
        if self.plain {
            "> "
        } else {
            "▶ "
        }
    }

    /// Severity glyph, e.g. `✔` / `✖`.
    pub fn sym(self, sev: u8) -> &'static str {
        match (self.plain, sev) {
            (false, 0) => "✔",
            (false, 2) => "▲",
            (false, 3 | 4) => "✖",
            (false, _) => "?",
            (true, 0) => "+",
            (true, 2) => "!",
            (true, 3 | 4) => "X",
            (true, _) => "?",
        }
    }

    pub fn bullet(self) -> &'static str {
        if self.plain {
            "-"
        } else {
            "›"
        }
    }

    pub fn dot(self) -> &'static str {
        if self.plain {
            "|"
        } else {
            "·"
        }
    }

    /// Filled / empty gauge cells.
    pub fn gauge_cells(self, mono: bool) -> (&'static str, &'static str) {
        // The kernel console font has no `━`: it falls back to `-` for both
        // cells and the gauge loses its filled/empty contrast. Blocks it has.
        match (self.plain, mono || kernel_console()) {
            (true, _) => ("#", "."),
            (false, false) => ("━", "─"),
            (false, true) => ("█", "░"),
        }
    }

    /// DIMM slot / CPU thread cells (used, free).
    pub fn slot_cells(self) -> (&'static str, &'static str) {
        if self.plain {
            ("#", ".")
        } else {
            ("■", "□")
        }
    }

    pub fn spinner(self, tick: u8) -> &'static str {
        if self.plain {
            SPIN_PLAIN[tick as usize % SPIN_PLAIN.len()]
        } else {
            SPIN_FANCY[tick as usize % SPIN_FANCY.len()]
        }
    }

    /// Breadcrumb / flow arrow (`▸` fancy, `>` plain).
    pub fn arrow(self) -> &'static str {
        if self.plain {
            ">"
        } else {
            "▸"
        }
    }

    pub fn degrees(self) -> &'static str {
        if self.plain {
            "C"
        } else {
            "°C"
        }
    }
}

/// `App` + [`Palette`] + [`Ui`]: everything a widget needs, resolved once.
#[derive(Debug, Clone)]
pub struct Theme {
    pub app: App,
    pub palette: Palette,
    pub ui: Ui,
}

impl Theme {
    /// Resolve from CLI `flags` and the process environment. See the
    /// [module docs](crate::theme) for the precedence.
    pub fn resolve(app: App, flags: Flags) -> Self {
        Self::from_env(
            app,
            flags,
            std::env::var_os("NO_COLOR").is_some(),
            own_color(&app).as_deref(),
            std::env::var("COLORTERM").ok().as_deref(),
        )
    }

    /// Pure constructor — no environment access.
    pub fn from_env(
        app: App,
        flags: Flags,
        no_color: bool,
        color: Option<&str>,
        colorterm: Option<&str>,
    ) -> Self {
        let mode = ColorMode::from_env(flags, no_color, color, colorterm);
        Theme {
            app,
            palette: Palette::new(mode, flags.light, flags.transparent),
            ui: Ui::new(flags.plain),
        }
    }

    pub fn is_mono(&self) -> bool {
        self.palette.is_mono()
    }

    pub fn is_plain(&self) -> bool {
        self.ui.plain
    }

    /// The focused-pane marker: `▸` normally; `>` in `--plain` **or mono** so
    /// focus never depends on colour. See [`crate::focus`].
    pub fn focus_mark(&self) -> &'static str {
        if self.ui.plain {
            ">"
        } else {
            "▸"
        }
    }

    /// The list-selection cursor: `▶` normally, `>` in `--plain`/mono.
    pub fn selection_mark(&self) -> &'static str {
        if self.ui.plain || self.is_mono() {
            "> "
        } else {
            "▶ "
        }
    }

    /// Truncate to `max` display chars with an ellipsis (`~` when plain).
    pub fn clip(&self, s: &str, max: usize) -> String {
        if s.chars().count() <= max {
            s.to_string()
        } else if max == 0 {
            String::new()
        } else {
            let mut out: String = s.chars().take(max - 1).collect();
            out.push(if self.ui.plain { '~' } else { '…' });
            out
        }
    }

    /// Breadcrumb `MODULE ▸ TAB ▸ (item)`; empty parts are dropped.
    pub fn breadcrumb(&self, parts: &[&str]) -> String {
        let sep = format!(" {} ", self.ui.arrow());
        parts
            .iter()
            .filter(|p| !p.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(&sep)
    }

    /// The product's pixel logotype, two rows of half-blocks.
    pub fn logo_lines(&self) -> Vec<ratatui::text::Line<'static>> {
        crate::widgets::logo_lines(self.app.brand, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neon_dark_palette_matches_the_spec() {
        let p = Palette::new(ColorMode::Neon, false, false);
        assert_eq!(p.accent, Color::Rgb(0, 229, 255));
        assert_eq!(p.accent2, Color::Rgb(255, 46, 151));
        assert_eq!(p.dim, Color::Rgb(112, 132, 152));
        assert_eq!(p.border, Color::Rgb(28, 74, 92));
        assert_eq!(p.track, Color::Rgb(34, 46, 60));
        assert_eq!(p.ok, Color::Rgb(57, 255, 136));
        assert_eq!(p.warn, Color::Rgb(255, 176, 0));
        assert_eq!(p.bad, Color::Rgb(255, 59, 59));
        assert_eq!(p.fg, Color::Rgb(196, 210, 224));
        assert_eq!(p.bg, Some(Color::Rgb(10, 14, 20)));
        assert_eq!(p.bar, Some(Color::Rgb(18, 26, 38)));
        assert_eq!(p.on_badge, Color::Rgb(10, 14, 20));
    }

    #[test]
    fn resolution_precedence() {
        let f = Flags::default();
        // NO_COLOR wins over everything colour-ish.
        assert_eq!(
            ColorMode::from_env(f, true, Some("truecolor"), Some("truecolor")),
            ColorMode::Mono
        );
        // An explicit `--mono` wins over NO_COLOR being absent / colour env.
        assert_eq!(
            ColorMode::from_env(Flags { mono: true, ..f }, false, Some("truecolor"), None),
            ColorMode::Mono
        );
        // The product's own override beats COLORTERM.
        assert_eq!(
            ColorMode::from_env(f, false, Some("ansi"), Some("truecolor")),
            ColorMode::Ansi
        );
        assert_eq!(
            ColorMode::from_env(f, false, Some("truecolor"), None),
            ColorMode::Neon
        );
        // COLORTERM is the fallback.
        assert_eq!(
            ColorMode::from_env(f, false, None, Some("24bit")),
            ColorMode::Neon
        );
        assert_eq!(ColorMode::from_env(f, false, None, None), ColorMode::Ansi);
        // Garbage override values fall through to COLORTERM.
        assert_eq!(
            ColorMode::from_env(f, false, Some("purple"), Some("truecolor")),
            ColorMode::Neon
        );
    }

    #[test]
    fn flags_shape_the_theme() {
        let t = Theme::from_env(
            WAYANG_FW,
            Flags {
                light: true,
                transparent: true,
                ..Flags::default()
            },
            false,
            Some("truecolor"),
            None,
        );
        assert_eq!(t.app, WAYANG_FW);
        assert_eq!(t.app.brand, "WAYANG FW");
        assert_eq!(t.palette.accent, Color::Rgb(0, 112, 160));
        assert!(t.palette.bg.is_none(), "transparent drops the background");
        assert!(!t.is_plain());
    }

    #[test]
    fn level_thresholds() {
        let p = Palette::new(ColorMode::Neon, false, false);
        assert_eq!(p.level(10.0, 70.0, 90.0), p.ok);
        assert_eq!(p.level(75.0, 70.0, 90.0), p.warn);
        assert_eq!(p.level(95.0, 70.0, 90.0), p.bad);
    }

    #[test]
    fn transparent_drops_background() {
        assert!(Palette::new(ColorMode::Neon, false, true).bg.is_none());
        assert!(Palette::new(ColorMode::Neon, false, false).bg.is_some());
    }

    #[test]
    fn breadcrumb_drops_empty_parts() {
        let t = Theme::from_env(WAYANG_ROUTER, Flags::default(), false, None, Some("24bit"));
        assert_eq!(t.breadcrumb(&["ROUTES", "", "eth0"]), "ROUTES ▸ eth0");
        let plain = Theme::from_env(
            WAYANG_ROUTER,
            Flags {
                plain: true,
                ..Flags::default()
            },
            false,
            None,
            Some("24bit"),
        );
        assert_eq!(plain.breadcrumb(&["A", "B"]), "A > B");
    }
}
