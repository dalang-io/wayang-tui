//! Terminal lifecycle, robustness and child-spawning helpers.
//!
//! Spec: `wayangos/docs/TUI-UX-REVAMP.md` §5b (no-flash startup & handoff)
//! and §5c (robustness against external tty output). Three concerns, one
//! place, so all HUDs behave identically:
//!
//! 1. **Lifecycle** — [`TermGuard`] enters the alternate screen, hides the
//!    cursor and themes the page background with **OSC 11** ([`osc11_bg`]), so
//!    the alternate screen is cleared to the palette `bg` instead of the
//!    terminal default (the light/SSH white flash). On drop — normal scope
//!    exit **or** a panic unwind — it shows the cursor, leaves the alternate
//!    screen and resets the background. [`enter`] / [`leave`] write the same
//!    bytes for callers that manage their own guard. (A literal
//!    `std::process::exit` bypasses `Drop`; install a panic hook / call
//!    [`leave`] yourself if you need it there.)
//! 2. **Self-healing redraw** — [`Repaint`] is a cheap "the next frame must be
//!    a full redraw" flag. Request it at startup, after returning from a child
//!    tool, on `SIGWINCH` and on a slow tick; [`Repaint::before_draw`] /
//!    [`Repaint::draw`] clear the terminal **only** when requested, so the
//!    normal path has no flicker.
//! 3. **No leaked child output** — [`command`] / [`spawn_cmd`] hand back a
//!    [`Cmd`] whose `stdout`/`stderr` default to [`Stdio::null`], so a child
//!    can never scribble over the HUD. [`command_capture`] / [`output_cmd`]
//!    pipe instead, for when the caller will read the result.
//!
//! Every escape string is a pure function and unit-tested; nothing in this
//! module touches a real terminal during tests.

use std::ffi::OsStr;
use std::io::{self, Write};
use std::ops::{Deref, DerefMut};
use std::process::{Child, Command, Output, Stdio};

use ratatui::backend::Backend;
use ratatui::style::Color;
use ratatui::Terminal;

use crate::theme::Palette;

// ─── OSC 11 / lifecycle sequences ───────────────────────────────────────────

/// Enter the alternate screen buffer.
pub const ENTER_ALT: &str = "\x1b[?1049h";
/// Leave the alternate screen buffer.
pub const LEAVE_ALT: &str = "\x1b[?1049l";
/// Hide the cursor.
pub const HIDE_CURSOR: &str = "\x1b[?25l";
/// Show the cursor.
pub const SHOW_CURSOR: &str = "\x1b[?25h";
/// OSC 111: reset the terminal background to its default.
pub const OSC11_RESET: &str = "\x1b]111\x1b\\";

/// The OSC 11 sequence that sets the terminal background to the palette `bg`,
/// or `None` when the palette has no true-colour background (mono,
/// `--transparent`, or a 16-colour ANSI palette — those keep the terminal's
/// own background).
///
/// Format is the xterm `rgb:RR/GG/BB` form terminated with `ST` (`ESC \`).
pub fn osc11_bg(palette: &Palette) -> Option<String> {
    match palette.bg {
        Some(Color::Rgb(r, g, b)) => Some(format!("\x1b]11;rgb:{r:02x}/{g:02x}/{b:02x}\x1b\\")),
        _ => None,
    }
}

/// The paired enter / leave escape payloads for a palette.
///
/// `enter` sets the background first (so the alternate screen is cleared to
/// it), then enters the alternate screen and hides the cursor. `leave` shows
/// the cursor, leaves the alternate screen, then resets the background only if
/// [`enter`](Self::enter) set one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermSequences {
    /// Bytes to write when taking over the terminal.
    pub enter: String,
    /// Bytes to write when giving it back.
    pub leave: String,
}

/// Build the [`TermSequences`] for `palette`.
pub fn sequences(palette: &Palette) -> TermSequences {
    let bg = osc11_bg(palette);
    let mut enter = String::new();
    if let Some(bg) = &bg {
        enter.push_str(bg);
    }
    enter.push_str(ENTER_ALT);
    enter.push_str(HIDE_CURSOR);

    let mut leave = String::from(SHOW_CURSOR);
    leave.push_str(LEAVE_ALT);
    if bg.is_some() {
        leave.push_str(OSC11_RESET);
    }
    TermSequences { enter, leave }
}

/// Take over the terminal: theme the background, enter the alternate screen,
/// hide the cursor. Pair with [`leave`] (or use [`TermGuard`]).
pub fn enter<W: Write + ?Sized>(out: &mut W, palette: &Palette) -> io::Result<()> {
    out.write_all(sequences(palette).enter.as_bytes())
}

/// Give the terminal back: show the cursor, leave the alternate screen, reset
/// the background if it was themed. Idempotent at the caller's discretion.
pub fn leave<W: Write + ?Sized>(out: &mut W, palette: &Palette) -> io::Result<()> {
    out.write_all(sequences(palette).leave.as_bytes())
}

/// RAII restore guard: [`TermGuard::enter`] writes the enter sequences and the
/// guard writes the leave sequences on drop (normal exit **and** panic unwind).
///
/// The writer is generic, so tests pass a `Vec<u8>` and products pass
/// `std::io::stdout()`. Drop ignores write errors (there is nowhere to report
/// them); call [`TermGuard::leave`] explicitly if you want them.
pub struct TermGuard<W: Write> {
    out: W,
    leave: Vec<u8>,
    active: bool,
}

impl<W: Write> TermGuard<W> {
    /// Enter the alternate screen on `out` and arm the restore guard.
    pub fn enter(mut out: W, palette: &Palette) -> io::Result<Self> {
        let seq = sequences(palette);
        out.write_all(seq.enter.as_bytes())?;
        out.flush()?;
        Ok(Self {
            out,
            leave: seq.leave.into_bytes(),
            active: true,
        })
    }

    /// Restore the terminal now. Idempotent; `Drop` calls it if it has not run.
    pub fn leave(&mut self) -> io::Result<()> {
        if self.active {
            self.active = false;
            self.out.write_all(&self.leave)?;
            self.out.flush()?;
        }
        Ok(())
    }

    /// Has the guard still to restore the terminal?
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl<W: Write> Drop for TermGuard<W> {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}

// ─── force repaint ──────────────────────────────────────────────────────────

/// A cheap "the next frame must be a full redraw" flag.
///
/// The incremental renderer leaves stale cells behind when something else has
/// written to the tty (kernel chatter, a child, a resize). Request a repaint
/// at those moments; the normal frame path stays flicker-free because
/// [`before_draw`](Self::before_draw) clears only when requested and then
/// consumes the flag.
///
/// Call [`request`](Self::request) on startup, after a child tool returns, on
/// `SIGWINCH`, and on a slow (≈2–5 s) tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Repaint {
    requested: bool,
}

impl Repaint {
    /// A flag with no repaint pending.
    pub const fn new() -> Self {
        Self { requested: false }
    }

    /// Request that the next frame be a full redraw.
    pub fn request(&mut self) {
        self.requested = true;
    }

    /// Is a full redraw pending?
    pub fn is_requested(&self) -> bool {
        self.requested
    }

    /// Consume the pending flag: `true` at most once per [`request`](Self::request).
    pub fn take(&mut self) -> bool {
        std::mem::take(&mut self.requested)
    }

    /// Clear `terminal` if a repaint was requested; returns whether it cleared.
    pub fn before_draw<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<bool> {
        if self.take() {
            terminal.clear()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// [`before_draw`](Self::before_draw), then draw: the one call a product's
    /// frame loop should use in place of `terminal.draw`.
    pub fn draw<B: Backend, F>(&mut self, terminal: &mut Terminal<B>, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.before_draw(terminal)?;
        terminal.draw(f).map(|_| ())
    }
}

// ─── child spawning ─────────────────────────────────────────────────────────

/// How a spawned child's `stdout`/`stderr` are wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildIo {
    /// Discard both streams (`Stdio::null`) — the default for HUD children, so
    /// they never write over the shared tty (§5c).
    Silent,
    /// Pipe both streams so the caller can read them ([`Stdio::piped`]).
    Capture,
    /// Inherit both streams ([`Stdio::inherit`]) — only for a genuinely
    /// interactive tool that the HUD has handed the terminal over to.
    Inherit,
}

impl ChildIo {
    fn stdio(self) -> (Stdio, Stdio) {
        match self {
            ChildIo::Silent => (Stdio::null(), Stdio::null()),
            ChildIo::Capture => (Stdio::piped(), Stdio::piped()),
            ChildIo::Inherit => (Stdio::inherit(), Stdio::inherit()),
        }
    }

    fn apply(self, cmd: &mut Command) {
        let (out, err) = self.stdio();
        cmd.stdout(out).stderr(err);
    }
}

/// A [`Command`] pre-configured so the child cannot scribble on the shared tty.
///
/// Derefs to [`Command`], so `.arg(..)`, `.env(..)`, `.status()` and friends
/// work as usual; use [`io`](Self::io) to inspect (and test) the configured
/// streams and [`into_command`](Self::into_command) to hand the bare `Command`
/// to code that takes one by value. Overriding `stdout`/`stderr` directly is
/// allowed but leaves the recorded [`ChildIo`] stale; prefer
/// [`silent`](Self::silent) / [`capture`](Self::capture).
#[derive(Debug)]
pub struct Cmd {
    inner: Command,
    io: ChildIo,
}

impl Cmd {
    fn new(program: impl AsRef<OsStr>, io: ChildIo) -> Self {
        let mut inner = Command::new(program);
        io.apply(&mut inner);
        Self { inner, io }
    }

    /// The configured child I/O policy.
    pub fn io(&self) -> ChildIo {
        self.io
    }

    /// Discard the child's `stdout`/`stderr` (the default).
    pub fn silent(mut self) -> Self {
        self.io = ChildIo::Silent;
        self.io.apply(&mut self.inner);
        self
    }

    /// Pipe the child's `stdout`/`stderr` for capture.
    pub fn capture(mut self) -> Self {
        self.io = ChildIo::Capture;
        self.io.apply(&mut self.inner);
        self
    }

    /// Consume into the underlying [`Command`].
    pub fn into_command(self) -> Command {
        self.inner
    }
}

impl Deref for Cmd {
    type Target = Command;

    fn deref(&self) -> &Command {
        &self.inner
    }
}

impl DerefMut for Cmd {
    fn deref_mut(&mut self) -> &mut Command {
        &mut self.inner
    }
}

impl From<Cmd> for Command {
    fn from(cmd: Cmd) -> Command {
        cmd.inner
    }
}

/// A [`Command`] with `stdout`/`stderr` discarded: the default for anything the
/// HUD spawns. Add args on the returned [`Cmd`].
pub fn command(program: impl AsRef<OsStr>) -> Cmd {
    Cmd::new(program, ChildIo::Silent)
}

/// A [`Command`] with `stdout`/`stderr` piped, for a caller that reads them
/// (e.g. parsing `nft`/`wg` output).
pub fn command_capture(program: impl AsRef<OsStr>) -> Cmd {
    Cmd::new(program, ChildIo::Capture)
}

/// Spawn `program` with `args`, output discarded. The convenience form of
/// `command(program).args(args).spawn()`.
pub fn spawn_cmd(program: &str, args: &[&str]) -> io::Result<Child> {
    let mut cmd = command(program);
    cmd.args(args.iter().copied());
    cmd.spawn()
}

/// Run `program` to completion with `args`, capturing stdout/stderr. The
/// convenience form of `command_capture(program).args(args).output()`.
pub fn output_cmd(program: &str, args: &[&str]) -> io::Result<Output> {
    let mut cmd = command_capture(program);
    cmd.args(args.iter().copied());
    cmd.output()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Flags, Theme, WAYANG_FW};

    fn neon() -> Theme {
        Theme::from_env(WAYANG_FW, Flags::default(), false, Some("truecolor"), None)
    }

    fn ansi() -> Theme {
        Theme::from_env(WAYANG_FW, Flags::default(), false, Some("ansi"), None)
    }

    fn mono() -> Theme {
        Theme::from_env(WAYANG_FW, Flags::default(), true, None, None)
    }

    #[test]
    fn osc11_is_the_spec_background() {
        let t = neon();
        assert_eq!(
            osc11_bg(&t.palette).as_deref(),
            Some("\x1b]11;rgb:0a/0e/14\x1b\\")
        );
        // Two-digit lowercase hex for every channel.
        let mut p = t.palette.clone();
        p.bg = Some(Color::Rgb(242, 246, 250));
        assert_eq!(osc11_bg(&p).as_deref(), Some("\x1b]11;rgb:f2/f6/fa\x1b\\"));
    }

    #[test]
    fn osc11_is_skipped_without_a_truecolour_background() {
        assert_eq!(osc11_bg(&mono().palette), None);
        assert_eq!(osc11_bg(&ansi().palette), None);
        let mut transparent = neon().palette;
        transparent.bg = None;
        assert_eq!(osc11_bg(&transparent), None);
    }

    #[test]
    fn sequences_enter_set_bg_then_alt_then_hide() {
        let t = neon();
        let seq = sequences(&t.palette);
        assert_eq!(
            seq.enter,
            format!("\x1b]11;rgb:0a/0e/14\x1b\\{ENTER_ALT}{HIDE_CURSOR}")
        );
        assert_eq!(seq.leave, format!("{SHOW_CURSOR}{LEAVE_ALT}{OSC11_RESET}"));
    }

    #[test]
    fn sequences_do_not_reset_a_background_they_did_not_set() {
        let seq = sequences(&ansi().palette);
        assert_eq!(seq.enter, format!("{ENTER_ALT}{HIDE_CURSOR}"));
        assert_eq!(seq.leave, format!("{SHOW_CURSOR}{LEAVE_ALT}"));
    }

    #[test]
    fn guard_writes_enter_then_leave_on_drop() {
        let mut buf: Vec<u8> = Vec::new();
        {
            let guard = TermGuard::enter(&mut buf, &neon().palette).unwrap();
            assert!(guard.is_active());
        }
        assert_eq!(
            String::from_utf8_lossy(&buf),
            format!("\x1b]11;rgb:0a/0e/14\x1b\\{ENTER_ALT}{HIDE_CURSOR}{SHOW_CURSOR}{LEAVE_ALT}{OSC11_RESET}")
        );
    }

    #[test]
    fn free_enter_and_leave_write_the_paired_sequences() {
        let mut buf: Vec<u8> = Vec::new();
        enter(&mut buf, &ansi().palette).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&buf),
            format!("{ENTER_ALT}{HIDE_CURSOR}")
        );
        leave(&mut buf, &ansi().palette).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&buf),
            format!("{ENTER_ALT}{HIDE_CURSOR}{SHOW_CURSOR}{LEAVE_ALT}")
        );
    }

    #[test]
    fn guard_leave_is_idempotent() {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut guard = TermGuard::enter(&mut buf, &ansi().palette).unwrap();
            guard.leave().unwrap();
            assert!(!guard.is_active());
            guard.leave().unwrap(); // a second explicit leave is a no-op
        }
        // Drop after the explicit leave must not write a second leave.
        assert_eq!(
            String::from_utf8_lossy(&buf),
            format!("{ENTER_ALT}{HIDE_CURSOR}{SHOW_CURSOR}{LEAVE_ALT}")
        );
    }

    fn terminal() -> Terminal<ratatui::backend::TestBackend> {
        Terminal::new(ratatui::backend::TestBackend::new(10, 3)).unwrap()
    }

    #[test]
    fn repaint_logic_is_cheap_and_one_shot() {
        let mut flag = Repaint::new();
        assert!(!flag.is_requested());
        flag.request();
        assert!(flag.is_requested());
        assert!(flag.take(), "claims the pending repaint");
        assert!(!flag.take(), "only once");
        assert!(!flag.is_requested());
    }

    #[test]
    fn before_draw_clears_only_when_requested() {
        let mut term = terminal();
        term.draw(|f| {
            let area = f.area();
            f.render_widget(ratatui::widgets::Paragraph::new("hello"), area);
        })
        .unwrap();

        let mut flag = Repaint::new();
        assert!(!flag.before_draw(&mut term).unwrap(), "no clear by default");
        assert_eq!(term.backend().buffer()[(0, 0)].symbol(), "h");

        flag.request();
        assert!(flag.before_draw(&mut term).unwrap(), "clears when asked");
        assert_eq!(term.backend().buffer()[(0, 0)].symbol(), " ");
        assert!(!flag.before_draw(&mut term).unwrap(), "flag consumed");
    }

    #[test]
    fn draw_wrapper_clears_then_draws() {
        let mut term = terminal();
        term.draw(|f| {
            let area = f.area();
            f.render_widget(ratatui::widgets::Paragraph::new("junk"), area);
        })
        .unwrap();
        let mut flag = Repaint::new();
        flag.request();
        flag.draw(&mut term, |f| {
            let area = f.area();
            f.render_widget(ratatui::widgets::Paragraph::new("ok"), area);
        })
        .unwrap();
        let buf = term.backend().buffer();
        let text: String = (0..3)
            .flat_map(|y| (0..10).map(move |x| buf[(x, y)].symbol().to_string()))
            .collect();
        assert!(text.starts_with("ok"), "junk overwritten: {text:?}");
    }

    #[test]
    fn command_defaults_to_null_stdio() {
        // Unit-level: inspect the recorded policy, never spawn a child.
        let c = command("nft");
        assert_eq!(c.io(), ChildIo::Silent);
        assert_eq!(c.get_program(), OsStr::new("nft"));
    }

    #[test]
    fn command_capture_pipes_and_is_overridable() {
        let c = command_capture("wg");
        assert_eq!(c.io(), ChildIo::Capture);
        let c = c.capture();
        assert_eq!(c.io(), ChildIo::Capture);
        let c = c.silent();
        assert_eq!(c.io(), ChildIo::Silent);
        // Deref exposes the normal `Command` builder.
        let mut c = command("nft");
        c.arg("-a");
        assert_eq!(c.get_args().count(), 1);
    }
}
