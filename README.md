# wayang-tui

Shared **ratatui component library** for the wayang HUDs — the `wayang` CLI,
`wayang-fw` and `wayang-router` (and, later, `dcheck`).

Goal: the three consoles look and behave **identically** because they render
through the *same* components — not because `theme.rs`/`widgets.rs` were copied
three times and drifted.

Canonical visual spec: `wayangos/docs/TUI-UX-REVAMP.md` (§5 "Visual spec" +
"Focused-pane highlight").

## Use

```toml
[dependencies]
wayang-tui = { git = "https://github.com/dalang-io/wayang-tui", tag = "v0.1.0" }
```

## Contents (planned)

| module | what |
|---|---|
| `theme` | `Palette`, `Ui` — neon/ansi/mono/light, `NO_COLOR`/`--plain`, width thresholds |
| `widgets` | `panel(title, focused, right)`, `caption`, `keycaps`, `header`, `footer`, `selection_row`, `badge`, `status`, `gauge`, `field`, `logo`, `clip`, `centered` |
| `focus` | active-pane model + the colour-free `▸`/dim rule |
| `overlay` | help / review / quick-jump frames |

Status: **scaffold** — components are being extracted from `wayang-fw` /
`wayang-router` after their UX wave lands, then the three products drop their
copies and depend on this crate.
