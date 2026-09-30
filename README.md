# wayang-tui

Shared **ratatui component library** for the wayang HUDs — the `wayang` CLI,
`wayang-fw` and `wayang-router` (and, later, `dcheck`).

Goal: the products look and behave **identically** because they render through
the *same* components — not because `theme.rs`/`widgets.rs` were copied three
times and drifted.

Canonical visual spec: `wayangos/docs/TUI-UX-REVAMP.md` (§5 "Visual spec" +
"Focused-pane highlight", §5b no-flash startup & handoff, §5c terminal
robustness, §5d one tab = one full-screen view).

## Use

```toml
[dependencies]
wayang-tui = { git = "https://github.com/dalang-io/wayang-tui", tag = "v0.2.0" }
```

Each product picks its [`App`] identity and resolves a [`Theme`]:

```rust
use wayang_tui::theme::{Flags, Theme, WAYANG_FW};
use wayang_tui::widgets::{self, Header};

let theme = Theme::resolve(WAYANG_FW, Flags::default());

// in draw():
// let inner = widgets::panel_focused(f, area, "TARGET", None, &theme);
// widgets::header(f, header_area, &Header {
//     breadcrumb: vec!["POLICIES", "FILTER"],
//     badge: Some((0, "OK")),
//     revision: Some(env!("CARGO_PKG_VERSION")),
//     ..Header::default()
// }, &theme);
```

## Module map

| module | what |
|---|---|
| `theme` | `App` (`WAYANG_FW` / `WAYANG_ROUTER` / `WAYANG_OS`), `Flags`, `ColorMode`, `Palette`, `Ui`, `Theme` — neon/ansi/mono/light, `NO_COLOR`/`--plain`/`--light`/`--mono`/`--transparent`, `<PREFIX>_COLOR` |
| `widgets` | `panel(title, right, focused)` / `panel_focused`, `caption`, `keycaps`, `header` (brand · breadcrumb · glyph+word badge · rev), `footer`, `selection_row`, `badge`, `status`, `gauge`, `gauge_cells_for`, `field`, `logo`/`logo_lines`, `clip`, `centered` |
| `focus` | active-pane model `Focus` + `FocusRing`, and the colour-free `▸`/`>` marker rule |
| `overlay` | help / REVIEW / quick-jump frame: title chrome, scroll region, footer keys |
| `term` | `TermGuard` (alt screen + hide cursor + **OSC 11** palette `bg`, restored on drop/panic), `Repaint` full-redraw flag, and `command`/`spawn_cmd` (children default to `Stdio::null`, `command_capture`/`output_cmd` pipe) |
| `splash` | `render` — first-frame logotype + `loading <tool>…` + spinner + breadcrumb, drawable before sampling |
| `transition` | `render` — `▸ launching <target>…` handoff/return frame |
| `layout` | `body`/`body_sized` → `Layout { tab_row, content, detail }`: one tab = one full-screen view with an optional fixed bottom DETAIL strip |

## Startup, robustness, layout

§5b/§5c/§5d of the spec live in `term`/`splash`/`transition`/`layout`:

```rust
use wayang_tui::term::{self, Repaint, TermGuard};
use wayang_tui::{layout, splash, transition};

// Take the terminal (theme the background with OSC 11, alt screen, hide cursor).
// `std::io::stdout()` is a handle, so the ratatui Terminal may hold its own.
let _guard = TermGuard::enter(std::io::stdout(), &theme.palette)?;
let mut repaint = Repaint::new(); // request() on startup / SIGWINCH / child return / slow tick

// First frame — never a blank alt screen:
repaint.draw(&mut terminal, |f| {
    let areas = layout::body(f.area(), false);
    splash::render(f, areas.content, "wayang-fw", Some("FIREWALL SYSTEM"), tick, &theme);
})?;

// Hand off to a sibling; the child's own first frame is its splash:
// transition::render(f, area, "launching", "wayang-router", &theme);
let _status = term::spawn_cmd("wayang-router", &[])?; // stdio is null — no tty scribble
// … back: repaint.request(), then transition::render(.., "loading", ..).

// `_guard` drops → show cursor, leave alt screen, reset background.
```

`Repaint::draw(&mut term, |f| …)` is the drop-in for `term.draw(…)`: it calls
`Terminal::clear()` only when a repaint was requested, so the steady-state frame
loop stays flicker-free. `TermGuard` restores on a panic unwind too; a literal
`std::process::exit` bypasses `Drop` (call `term::leave` in a panic hook if you
need that path).

## Design notes

* **Per-product is a parameter.** `App { env_prefix, tool, brand }` carries the
  env-var prefix (`WAYANG_FW_COLOR` vs `WAYANG_ROUTER_COLOR`), the tool name and
  the **logo word** (`WAYANG FW` / `WAYANG ROUTER` / `WAYANG OS`) — nothing is
  hard-coded.
* **Colour is never the only signal.** Focus is `▸` (unfocused: none), badges
  are glyph **and** word (mono `[✔ OK]`), selection rows carry a cursor marker.
  In `NO_COLOR`/`--plain` the focus marker becomes `>` and bold still carries
  it.
* **One entry point.** `Theme::resolve(app, flags)` reads the environment;
  `Theme::from_env(app, flags, no_color, color, colorterm)` is the pure
  equivalent used by tests.
* Palette RGBs are fixed by the spec and asserted in `theme::tests`.

## Test

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Status

`v0.2.0` — adds the §5b/§5c/§5d building blocks (`term`, `splash`,
`transition`, `layout`) on top of the `v0.1.x` components. `v0.1.0` extracted
`theme`/`widgets`/`focus`/`overlay` from `wayang-fw` / `wayang-router` (and the
`wayang` CLI HUD). Products still carry their copies; migrating them onto this
crate — and wiring `TermGuard`/`splash`/`Repaint`/`layout::body` into their frame
loops — is a follow-up.
