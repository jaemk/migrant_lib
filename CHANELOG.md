# Changelog

## [Unreleased]
### Added

### Changed

### Removed

----

## [0.18.0]
### Added
- Add option to `Migrator` for suppressing output

### Changed
- Change `DbConn` to `ConnConfig`
    - Remove functionality for opening database connections
    - Add methods for getting database type and connection string
- Prevent embedded and function migrations from compiling without
  database features

### Removed
- Remove re-exported database connection crates

----

## [0.17.3]
### Added
- Add missing `Settings::configure_mysql` method
- Add completion message to `test.sh` script

### Changed
- Use `AsRef` trait in `Config::use_migrations`
- Update crate doc / readme
- Update lots of documentation all over
- Update contributing

### Removed

----

## [0.17.2]
### Added

### Changed
- Add link to contributing in crate doc and readme
- Update `embedded_programmable` example
    - Clean up unnecessary db feature cfg's
    - Make migrations more interesting
- Update docs using `include_str`

### Removed

----

## [0.17.1]
### Added

### Changed
- Fix mysql bug (when wrapping `mysqlsh`) where `can_connect`
  was returning an error on successful connections because
  `mysqlsh` stdout is empty

### Removed

----

## [0.17.0]
### Added
- Add MySQL support
    - drivers:
        - the `mysql` crate
        - wrapping the `mysqlsh` (mysql-shell) tool
- Add `d-all` feature to include all backends

### Changed
- Update `test.sh` script to deal with mysql setup/teardown
    - a `mysql` root password is required when running locally
    - when running on ci (travis), no password is required
    - shorten testing user name
- Add `mysql` re-export in the `types` module
- Update ci intall script to download/install `mysqlsh` (mysql-shell)
- Change feature flags to:
    - `d-postgres`
    - `d-sqlite`
    - `d-mysql`
    - `d-all`

### Removed

----

## [0.16.2]
### Added

### Changed
- Fix postgres default port bug

### Removed

----

## [0.16.1]
### Added
- Add top level reference to the migrant CLI tool in the readme
- Add a CONTRIBUTING.md

### Changed
- Exclude `Migrant.toml` testing settings file and `.travis.yml` in Cargo.toml

### Removed

----

## [0.16.0]
### Added
- Add ability to configure database specific options when initializing a
  new settings file from the settings templates
- Add additional configurable template params to postgres settings template
- Add notes in `FileMigration` docs on what relative path definitions are
  relative to

### Changed
- Rename `ConfigInitializer` to `SettingsFileInitializer`
- Store extra database connection params in a `BTreeMap` so they come out ordered
- Update examples
- Update docs
- Update readme
- Exclude "ci/" dir in Cargo.toml
- Fix travis ci build status link in readme & crate doc

### Removed

----

## [0.15.1]
### Added
- Add `ConfigInitializer::migration_location` to override default `migration_location`
  in config file generated templates.

### Changed
- Update `migrant_cli_compatible` example to look in the `managed` migrations dir
  so it sees proper migrant-generated tags/filenames.
- Convert remaining public signatures that took `Path`/`PathBuf` to `T: AsRef<Path>`
- Deprecate `ConfigInitializer::for_database`
- Add `ConfigInitializer::database_type` to replace `ConfigInitializer::for_database`

### Removed

----

## [0.15.0]
### Added
- Add specific settings builders per database type
- Update internal handling of settings

### Changed
- Replace `Settings::with_db_type` with `Settings::configure_<dbtype>` methods
- Update settings
- Update postgres settings file template
- Update test.sh script to exit with an error when `cargo test` fails
- Update `ConfigInitializer::for_database` to take a `DbKind` instead of `str`
- Update `Migrator` api to use mut refs instead of passing ownership

### Removed
- `Settings::with_db_type`

----

## [0.14.0]
### Added
- Explicit & configurable `Settings` struct.
    - These are the configurable settings used by the `Config` type
      are were previously only configurable in a file
    - Migrant.toml config files can be replaced by `Settings` configured in source.
- `Config::with_settings` for initializing a `Config` from `Settings`

### Changed
- Config file renamed from `.migrant.toml` to `Migrant.toml`
    - In sqlite configs, `database_name` parameter is now `database_path`
      and can be either an absolute or relative (to the config file dir) path.
    - Config file must be renamed (and `database_name` changed to `database_path`)
      or re-initialized.
- `Config::load_file_only` renamed to `Config::from_settings_file`
- `search_for_config` renamed to `search_for_settings_file`
- Output from `Config::setup` is now only shown in debug logs (`debug!` macro)
- Move to separate repo (apart from `migrant` the cli tool)

### Removed

