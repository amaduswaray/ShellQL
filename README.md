
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

## Future todos:
- [ ] add `shql <string>` that lets you run shql and string and imediately go into a connection without saving it or persiting it. Simply to view db
  - [ ] Useful for piping strings or just running

- [ ] Create trait that holds common functions for models(host, connection validator, connection builder, connection string extractor, etc)
- [ ] Session manager - tmux lik sessionizer to change between connctions. Makes connections out of scope and harder to accidentaly change wrong dbs
- [ ] Templates: Be able to save pane layouts for quick access of views you often use
- [ ] Bounding all views to a singular table. can be changed from tableList, or the :open command. All panes are connected to the same table
  - [ ] Idea/NOTE: a window should always be connected to a singular table. New table always opens a new window
- [ ] Windows, sessions: Open new windows for multiple tabs of a singular connection, or have sessions with multiple connections

- [ ] Run query from CLI
  - [ ] Flags needed to pick from existing db, but can be powerful with shell pipes
  - [ ] Should be able to pass in sql file path too


### TODOS TUI

- [x] For new table or shcema, if table dont exist, give error, dont create pane
- [x] sql queries and testing. Boot up docker compose file with seed script, and run tests on the data
- [x] resizing panes
- [x] :q when 1 pane left should exit
- [x] :q in home view should exit
- [x] Filter and sort logic
- [x] Search logic when in files. vim like search
- [x] Visual mode proper for row and column Visual mode proper for row and columnss
- [x] :exit command to take user to homescreen
- [x] fix new pane function when having 1 vertical split and trying to make a new vsplit
- [x] Fuzzy find on search
  - [x] Live matching of results
  - [x] cmdline saying n/m results
- [x] shift+k for hover feature. Puts the value in the cmd line so that its readable
- [x] Alternating row background color to look more like excel
- [x] :Disconnect command
- [x] :connect command to connect to a new db(session based)

- [x] Proper SQL syntax that works like sqlit
- [x] Editor commands and working with proper syntax, ans not "" stuff
- [x] Syntax highlighting and editor formatting

- [ ] Complete the command line
  - [ ] All commands like, new session, switch session, delete session, add connection, etc
  - [ ] Tab completion for command line commands
- [ ] Be able to Delete when in connection list
- [ ] Auto complete for tables and commands
- [ ] Inline cell edits
- [ ] Proper vim commands
  - [ ] o and O for newline over and under, to add a new row
  - [ ] ciw to change in cell
  - [ ] rn to rename the cell value
  - [ ] i to enter insert mode and edit

