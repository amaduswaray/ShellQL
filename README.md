# ShellQL

> **ShellQL is a database manager TUI for developers.**
>
> It is Vim- and tmux-inspired, built for ergonomic SQL workflows and database management from the terminal.
> If you know Vim and SQL, you will feel right at home.

---

##  Quick Start

Get up and running in minutes:

### Install

```bash
cargo install shellql
```

### Launch ShellQL

```bash
shql
```

### Connect to a database

```bash
shql connect --interactive
```

### Run a sample query

```sql
SELECT * FROM users;
```

---

##  Getting Started

ShellQL is a keyboard-first terminal application for working with databases directly from your shell.

It is designed for developers who want speed, efficiency, and full keyboard control without relying on heavy GUI tools.

---

##  Beta Status

ShellQL is currently in beta.

It is already usable for daily workflows, but features, UX improvements, and keybindings are actively evolving.

---

##  Core Concepts

### Tabs
- Development
- Staging
- Production
- Schema exploration

### Panes
Each tab can be split into multiple panes for parallel workflows.

### Views
- Tables
- Table data
- Schema
- SQL editor
- Query results

---

##  Features

- Connection management (TUI + CLI)
- PostgreSQL, MySQL, SQLite support
- Vim-like SQL editor
- Context-aware autocomplete
- Table workflows (filter, sort, edit, delete, insert)
- Schema exploration
- Multi-pane & multi-tab workspace
- Query execution with live results

---

##  Installation Options

### Homebrew

```bash
brew tap amaduswaray/tap
brew install shellql
```

### Cargo

```bash
cargo install shellql
```

### Build from Source

```bash
git clone https://github.com/amaduswaray/ShellQL.git
cd shellql
cargo build --release
./target/release/shql
```

---

## Usage

### Launch TUI

```bash
shql
```

### CLI Commands

```bash
shql db add --name dev --engine postgres --url 'postgres://user:pass@localhost:5432/mydb'
shql db list
shql db delete --name dev
shql connect --interactive
```

---

##  Demos

ShellQL in action:

- Overview of UI
- Adding a connection
- Pane workflows
- Tab navigation
- Column search
- Filtering and sorting
- Column projection
- Multiple queries
- SQL execution

---

##  Keybindings

### Navigation
- j/k move
- h/l navigate
- Ctrl+h/j/k/l pane focus

### Actions
- Enter select
- : command line
- / search
- q quit

### Table
- i edit
- dd delete
- o/O insert
- u undo
- :w commit

### Editor
- Vim-style motions supported
- SQL autocomplete enabled

---

##  Development Setup

```bash
git clone https://github.com/amaduswaray/ShellQL.git
cd shellql
cargo build
cargo run
```

---

##  Contributing

1. Fork repo
2. Create branch

```bash
git checkout -b feature-name
```

3. Make changes
4. Test locally
5. Commit

```bash
git commit -m "Describe change"
```

6. Push

```bash
git push origin feature-name
```

7. Open PR

---

##  Docs
- [Documentation](./docs/documentation.md)
- [Homebrew Guide](./docs/homebrew.md)
- [Contributing Guide](./CONTRIBUTING.md)

---

##  Versioning
- v0.1.x-beta

---

##  License
MIT
