# wayang-tui

Shared **ratatui component library** for the wayang HUDs — the `wayang` CLI,
`wayang-fw` and `wayang-router` (and, later, `dcheck`).

Goal: the products look and behave **identically** because they render through
the *same* components — not because `theme.rs`/`widgets.rs` were copied three
times and drifted.

Canonical visual spec: `wayangos/docs/TUI-UX-REVAMP.md` (§5 "Visual spec" +
"Focused-pane highlight").

## Use

```toml
[dependencies]
wayang-tui = { git = "https://github.com/dalang-io/wayang-tui", tag = "v0.1.0" }
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

`v0.1.0` — components extracted from `wayang-fw` / `wayang-router` (and the
`wayang` CLI HUD). Products still carry their copies; migrating them onto this
crate is a follow-up.
