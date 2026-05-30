
# ShellQL

The next generation database manager

Shellql allows you to connect with your favouirite db

From the terminal, do any relevant db operation without having to boot up an instance

## Descpription

A database manager for postgresql, mysql, sqlite and mongodb.

This lets you view schemas, edit tables and run queries without needing a separate paid application

## Installation

For mac, you can use homebrew

ShellQL is a db manager made for the terminal.
Connect to your favourite provider and handle tables and schemas in your db

## V0.1 Issues

- [ ] Inline cell edits in tableview
- [ ] cmdline query always pops up result view
- [ ] completions and its scrollwheel
- [ ] zsh Shell completions for the cli commands
- [ ] query history popup pane
- [ ] Better result view and query cohesion
- [ ] add more posix compliant cli commands - Makes pipable workflow better
- [ ] Save layouts
- [ ] No general trait for easy driver expansion
- [ ] picker for tabs
- [ ] Tmux like session management for multple connections
- [ ] More expressive cli that integrates better with unix and AI
- [ ] In app documentation and help
- [ ] Adding custom keybinds 
- [ ] Leader key
- [ ] Advanced vim bindings
- [ ] Drivers
  - [ ] turso/libsql
  - [ ] mariadb
  - [ ] mongodb
  - [ ] Cludflare D1
  - [ ] More....
- [ ] More Pane options
  - [ ] Indexes
  - [ ] Views


### TODOS TUI

- [x] Fix tab/pane deletion
- [x] Fix deletion logic - dd should prompt a deletion confirm tab
- [x] :schema or new :schema with no following param should open a table list, then you can select which table to open the schema for
- [x] fix schema bug
- [x] for border overflows and showing(add connection)
- [x] Proper vim commands
  - [x] o and O for newline over and under, to add a new row
  - [x] navigate text in the cmdline with arrows

- [x] Vim based query editor
  - [x] Remember visual mode
  - [x] Cursor edit
  - [x] commands like dG dgg, yG, etc

- [ ] Tests for the new sql functions
- [ ] Minor improvements for colors and such
- [ ] Complete the command line commands
