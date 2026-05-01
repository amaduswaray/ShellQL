
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
- [x] Connect form
  - [x] Insert and normal mode for input fields so navigation works
  - [ ] Implement vim mode commands when in normal mode


