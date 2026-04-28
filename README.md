
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

- [ ] Write TESTTS!!
- [ ] Proper error handling and error boundaries
- [ ] Session manager - tmux lik sessionizer to change between connctions. Makes connections out of scope and harder to accidentaly change wrong dbs




### TODOS
- [x] Create new cli command for list
- [x] Update connections to also contain name
- [x] Add name option to connect cli command
- [x] Change the `list` cli arg to be a subset of `connections`. The prefix should be db
  - [x] Same with delete, add

- [ ] remove ID and use name as identifier
- [ ] Create cli steps -> Allow user to fill inn missing optionsConnect
  - [ ] if connect with no other flags, cli asks for all
  - [ ] Otherwise, ask for the missing fields(like connection string or name)


- [ ] Fix the interactive clipvoard paste issue
- [x] Add preview text to the cli interactive mode
- [x] Add description to all the commands and params
- [ ] Fix the dupe situation. Connections strings should be comparable
- [ ] Write tests for all the current functions
  - [ ] Connecting with a string
  - [ ] Not connecting on valid string
  - [ ] Validating a string
  - [ ] Not validating a string
  - [ ] Crud operations on db
