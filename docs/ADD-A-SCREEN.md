# Adding a screen (developer guide)

How a console screen is built in this crate's model. Read `README.md` and the
spec `wayangos/docs/TUI-UX-REVAMP.md` (§5 visual spec, §5b/§5c terminal, §5d
layout) first. The three products — `wayang`, `wayang-fw`, `wayang-router` —
all use this crate, so a screen built here behaves the same everywhere.

## The shape of a screen

```
header      brand · breadcrumb (MODULE ▸ TAB ▸ item) · state badge · revision
tab row     only when the screen has >1 view            (←→ switches)
content     the SELECTED tab, full width            ← one tab = one full screen
detail      shared bottom strip, only if a row has details
footer      the ≤6 keys for this context + status message
```

Use [`layout::body`](../src/layout.rs) to get the rects:

```rust
let l = layout::body(f.area(), has_detail);
let _ = widgets::header(f, l.header, &hdr, theme);
render_tab_row(f, l.tab_row, &tabs, active, theme);
render_selected_tab(f, l.content, …);          // full-screen
if let Some(d) = l.detail { render_detail(f, d, …); }
```

Never lay panes side-by-side or stacked per screen; the tab owns `content`.

## Recipe

1. **State.** Add the screen's variant/index to the app and, if it has views, an
   active-tab index. Track a selection per tab if needed.
2. **Navigation.** `←→` / `tab`/`shift-tab` change the active tab; `↑↓` move the
   selection; `enter` opens/edits/**stages**; `esc` goes back one level (and
   quits at the home deck); `/` jumps; `?` opens help. Keep this file's
   `on_key` the single dispatcher; a modal (overlay/form/review) takes priority.
3. **Render.** Draw panels with `widgets::panel(title, right, focused, theme)`
   and `widgets::panel_focused(...)` for the pane that owns the keys — exactly
   one is focused; it gets `accent` + a `▸` title prefix (`>` only with
   `--plain`). Tables/rows use `field`, `status`/`badge`, `caption`, `keycaps`,
   `gauge`, `clip`, `centered`.
4. **Footer & help.** Footer = the ≤6 most relevant keys (`widgets::footer` /
   `keycaps`). Add every key to the `?` help (`widgets`/`overlay`), grouped
   global/list/form/commit; `p` writes `keys.txt`.
5. **Actions.** Staging always goes through **REVIEW** (`overlay::Overlay`):
   show the exact effect (rendered rule / plan lines / candidate diff); `enter`
   applies, `esc` cancels. Committing is a separate, deliberate step with
   commit-confirm — never apply silently, never hide a side effect.

## Terminal lifecycle (do this once, in `main`)

* Enter with `term::TermGuard::enter(std::io::stdout(), &theme.palette)` and hold
  it for the HUD's lifetime; install a panic hook that calls `term::leave`.
  This sets the **OSC-11** background (no white flash) and the alt screen.
* Draw the **splash first** (`splash::render`) before sampling; on return from a
  child tool draw `transition::render(..., "loading", …)` then the splash.
* Drive frames through a `term::Repaint` and `repaint.draw(&mut terminal, …)`;
  `request()` at startup, on `SIGWINCH`, on child return, and on a ~2–5 s tick;
  bind `Ctrl-L` to `request()`. This is what makes the UI self-heal when
  something else writes to the tty.
* Run every `std::process::Command` through `term::command`/`spawn_cmd`/
  `output_cmd` so a child never scribbles on the shared terminal.

## Tests & screenshots

* **Nav invariants**: every screen with panes has a tab row; `←→` never leaves
  the module; `?` lists the keys.
* **Focus without colour**: render in `Palette::mono` and assert the focused
  title carries `▸` and the others do not.
* **Review**: staging shows the effect; `esc` returns to the form unchanged.
* **Snapshots**: render every screen with the product's `--screens`/`snapshot`
  command and commit the output; add a test that regenerates and compares so the
  checked-in screens never drift.

## Conventions (don't regress)

1. One tab = one **full-screen** view.
2. Exactly one focused pane, marked with `▸` **and** colour — readable with none.
3. Footer shows the current keys, never the whole map.
4. Staging ≠ committing; commit-confirm is mandatory; nothing applies silently.
5. Console/light/mono are first-class, not afterthoughts (`Ui::plain`,
   `Palette::mode`).
