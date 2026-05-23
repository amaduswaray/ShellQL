
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

- [x] Ensure that absolute paths for sqlite works. ensure sqlite and mysql works
- [x] When writing :! you should be able to write one liners sql queruers, limited to 1 select that can be displayed/ one query. 

- [ ] Delete connection in dashboard should not be possible
- [ ] query history popup pane
- [ ] for border overflows and showing(add connection)
- [ ] change from :open to :show

- [ ] When i visual mode, the alternating lites get removed to have a better highlight
- [ ] Selection highlight different color
- [ ] line number column does not need alternating lines
- [x] proper cmdline design

- [x] Full screen pane logic - like tmux <leader>z, makes a pane fullscreen. effect reverted when toggling or attempting to change pane

- [ ] Query view when wrong command shows result view
- [x] :select command that reduces the rows that are shown
- [x] :select, :where and :order should work in sequence. So all changes are persintant

- [ ] do a find buffer picker like vim
- [ ] :q when multiple tabs should not quit when pane in tab is 1. it should close the tab
- [ ] Tab logic, as sessions/picker they need to swap between. No visual tabs


- [ ] Complete the command line
  - [ ] All commands like, new session, switch session, delete session, add connection, etc
  - [ ] Tab completion for command line commands
  - [ ] Scrollable cmdline completions

- [ ] Be able to Delete when in connection list
- [x] Auto complete for tables and commands
- [x] Better navigation ergonomics: Using - to go back between views, or what not

- [ ] Inline cell edits. Only for tableView. the queryresults is read only, and can only use / and ?
- [ ] Proper vim commands
  - [ ] o and O for newline over and under, to add a new row
  - [ ] ciw to change in cell
  - [ ] rn to rename the cell value
  - [ ] i to enter insert mode and edit

- [ ] zsh Shell completions for the cli commands
- [ ] add more posix compliant 
