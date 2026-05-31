# ShellQL Documentation (Beta)

This document is the detailed usage reference for ShellQL.

- Project overview: [README.md](../README.md)
- Status: **beta**

---

## 1) Mental model

ShellQL has three core building blocks:

1. **Tabs**: separate work contexts
2. **Panes**: split areas inside a tab
3. **Views**: what each pane displays

Available view types:
- `tables` (TableList)
- `table` (TableView)
- `schema` (SchemaView / SchemaPicker)
- `editor` (SQL editor)
- `results` (query results)

This lets you build a task-focused workspace instead of hopping between separate windows/apps.

---

## 2) Supported databases

Current support:
- PostgreSQL
- MySQL
- SQLite

---

## 3) Running ShellQL

### TUI

```bash
shql
```

### CLI

```bash
# interactive connect flow
shql connect --interactive

# add/list/delete saved connections
shql db add --name dev --engine postgres --url 'postgres://user:pass@localhost:5432/mydb'
shql db list
shql db delete --name dev
```

---

## 4) Global controls

- `Ctrl+C` — force quit
- `:` — open command line (where supported)
- `Esc` — close/exit current prompt or mode

---

## 5) Home screen keybindings

- `j / k` or `↓ / ↑` — move selection
- `Enter` — connect to selected connection
- `c` — open connection picker
- `a` — add connection
- `d` — delete selected connection (confirm)
- `:` — command line
- `?` — help
- `q` — quit

---

## 6) Dashboard keybindings

### Navigation
- `h / j / k / l` or arrows — move cursor/navigation
- `gg` — top
- `G` — bottom
- `Ctrl+U` — half-page up
- `Ctrl+D` — half-page down

### Pane focus
- `Ctrl+h/j/k/l`
- `Ctrl+←/↓/↑/→`

### Search
- `/` — forward search
- `?` — backward search
- `n` — next match
- `N` — previous match
- `:noh` — clear highlights

### Mode / editing actions
- `i` — edit cell (TableView) or enter insert mode (Editor)
- `v` / `V` — visual row mode
- `Ctrl+v` — visual column mode
- `dd` — stage delete row (TableView)
- `d` + visual selection — stage delete selection
- `o` / `O` — stage insert row below/above
- `u` — undo staged change
- `:w` — commit staged changes
- `Tab` — next result set (Results view)
- `Shift+Tab` — previous result set (Results view)

### Misc
- `K` — cell peek
- `-` — pane history back
- `_` — pane history forward

---

## 7) Query editor behavior

The editor is Vim-inspired and supports:
- Normal/Insert/Visual modes
- motions/operators/text objects
- yank/delete/change + paste behavior
- yank flash feedback
- mode-based cursor shape (terminal support dependent)

Examples of supported actions:
- `dd`, `dw`, `diw`, `daw`
- `dG`, `dgg`
- `yy`, `yG`, `ygg`
- `p`, `P`

### Editor autocomplete

Autocomplete in the editor includes:
- **Table names** (especially after `FROM/JOIN/INTO/UPDATE/TABLE`)
- **SQL keywords** (prefix matched: `s -> select`, `f -> from`, etc.)

Popup behavior:
- no border
- muted background for contrast
- selection highlight
- scrollbar for long lists

---

## 8) Results view (single + multi-select)

When a query contains multiple statements (for example multiple `SELECT`s separated by `;`), ShellQL stores each statement output as a separate result set.

In the Results view:
- `Tab` moves to the next result set
- `Shift+Tab` moves to the previous result set
- the pane title shows your position as `Result n/m`

If your SQL has only one statement, it behaves like a normal single result table.

---

## 9) Command line commands (`:`)

### Home commands
- `:add`
- `:connect`
- `:d <name>` / `:delete <name>`
- `:help` / `:h`
- `:q` / `:quit` / `:exit`

### Dashboard commands

### Layout / navigation
- `:new tab`
- `:new pane [tables|table|schema|editor|results]`
- `:split`
- `:vsplit`
- `:hsplit`
- `:tab <id|next|prev|close>`
- `:q`
- `:close`
- `:full`

### View switching
- `:tables`
- `:table <name>`
- `:schema [table]`
- `:editor`
- `:results`

### Data operations (**TableView only**)
- `:where <expr>`
- `:order <column> [desc]`
- `:select <columns>`
- `:insert [above|below]`
- `:reset`
- `:w`

### Other
- `:! <sql>`
- `:connect`
- `:disconnect`
- `:back`
- `:forward`
- `:resize <direction> <amount>`
- `:help` / `:h`
- `:exit`

Notes:
- `:q` closes pane first, then tab, then app quit when appropriate
- `:delete` requires an explicit name

---

## 10) TableView workflow

TableView is the most powerful operational view and supports staged changes.

Typical flow:

1. Open table (`:table users`)
2. Narrow data (`:where ...`, `:order ...`, `:select ...`)
3. Edit cells (`i`)
4. Stage inserts (`o`/`O` or `:insert`)
5. Stage deletes (`dd` or visual + `d`)
6. Commit (`:w`)

Commit-time checks include required-value validation for inserts.

---

## 11) Search behavior

- `/` and `?` show live match counts as `[n/m]`
- count is aligned right in the cmdline search bar
- `n/N` navigate matches
- `:noh` clears highlights

---

## 12) Beta notes

ShellQL is moving quickly toward production readiness.

Release/tag convention for the beta line:
- `v0.1.x-beta` (Git tag)
- `0.1.x-beta` (`Cargo.toml`)
- increment `x` for each beta release
- helper script: `./scripts/bump-beta.sh`

Planned project hardening (outside this doc):
- issue tracker structuring
- packaging polish and release pipelines

---

## 13) Troubleshooting

If something looks off:

1. Check current mode (`Normal`, `Insert`, visual)
2. Verify active pane type (some commands are TableView-only)
3. Use `?` help overlays for in-app references
4. Re-run with latest build if you are tracking beta changes frequently

---

## 14) Project tone

ShellQL intentionally prioritizes keyboard fluency and composable workflows over hand-holding.

If you are comfortable with Vim motions and SQL, ShellQL should feel natural and fast.
