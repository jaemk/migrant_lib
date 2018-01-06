# Changelog

## [Unreleased]
### Added

### Changed

### Removed


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

