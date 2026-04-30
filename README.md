
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

- [ ] Raw sql files for premade queries
- [ ] Colors in general to output
- [ ] Prettyfy the error messages, like the rust compiler
- [ ] Write TESTTS!!
- [ ] Session manager - tmux lik sessionizer to change between connctions. Makes connections out of scope and harder to accidentaly change wrong dbs




### TODOS CLI
- [x] Create new cli command for list
- [x] Update connections to also contain name
- [x] Add name option to connect cli command
- [x] Change the `list` cli arg to be a subset of `connections`. The prefix should be db
  - [x] Same with delete, add

- [x] remove ID and use name as identifier
- [x] Create cli steps -> Allow user to fill inn missing optionsConnect
  - [x] if connect with no other flags, cli asks for all
  - [x] Otherwise, ask for the missing fields(like connection string or name)


- [x] Add -i flag to explicit set interactive. Otherwise the flags should be strict
- [x] Add preview text to the cli interactive mode
- [x] Add description to all the commands and params
- [x] Fix the dupe situation. Connections strings should be comparable

- [ ] Fix the interactive clipboard paste issue

- [ ] Write tests for all the current functions
  - [ ] Connecting with a string
  - [ ] Not connecting on valid string
  - [ ] Validating a string
  - [ ] Not validating a string
  - [ ] Crud operations on db
