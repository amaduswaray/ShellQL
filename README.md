
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

- [ ] Raw sql files for pre-made queries
- [ ] Create trait that holds common functions for models(host, connection validator, connection builder, connection string extractor, etc)
- [ ] Write TESTTS!!
- [ ] Session manager - tmux lik sessionizer to change between connctions. Makes connections out of scope and harder to accidentaly change wrong dbs


### TODOS TUI

- [ ] Complete the command line
  - [ ] All commands like, new session, switch session, delete session, add connection, etc
  - [ ] When an error is there, enter should take you out of the command line mode
  - [ ] Tab completion for command line commands
- [ ] Delete when in connection list
- [ ] For new table or shcema, if table dont exist, give error, dont create pane
  - [ ] Auto complete on table rows
- [ ] sql queries and testing. Boot up docker compose file with seed script, and run tests on the data


- [ ] Moving panes around
- [ ] resizing panes
- [ ] :q when 1 pane left should exit
- [ ] :q in home view should exit

- [ ] Filter and sort logic
  - [ ] use queres
  - [ ] How to filter with multiple columns? what is ergonomic
- [x] Search logic when in files. vim like search
- [ ] Visual mode proper for row and column Visual mode proper for row and columnss
- [ ] Windows, sessions
- [x] :exit command to take user to homescreen
- [ ] fix new pane function when having 1 vertical split and trying to make a new vsplit
