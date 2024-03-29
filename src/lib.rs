/*!
[![Build Status](https://travis-ci.org/jaemk/migrant_lib.svg?branch=master)](https://travis-ci.org/jaemk/migrant_lib)
[![crates.io:migrant_lib](https://img.shields.io/crates/v/migrant_lib.svg?label=migrant_lib)](https://crates.io/crates/migrant_lib)
[![docs](https://docs.rs/migrant_lib/badge.svg)](https://docs.rs/migrant_lib)

> Embeddable migration management
>
> Also see [`migrant`](https://github.com/jaemk/migrant) CLI

`migrant_lib` allows defining and embedding management of database migrations and
(connection) configuration in your compiled application.


**Available Features:**

| Feature       |    Backend                   |
|---------------|------------------------------|
| `d-postgres`  | Enable postgres connectivity |
| `d-sqlite`    | Enable sqlite connectivity   |
| `d-mysql`     | Enable mysql connectivity    |
| `d-all`       | Enable all backends          |


*Notes:*

- No features are enabled by default
- As of `0.20.0` the `d-sqlite` feature does not use `rusqlite`s `bundled` feature.
  If you would like `sqlite` to be bundled with your application, you will have to
  include `rusqlite` and enable the `bundled` feature in your project.


## Usage

- Migrations can be defined as files, string literals, or functions.
- File migrations can be either read from files at runtime or embedded in your executable at compile time
  (using [`include_str!`](https://doc.rust-lang.org/std/macro.include_str.html)).
- Migration tags must all be unique and may only contain the characters `[a-z0-9-]`.
  When running in a `cli_compatible` mode (see `Config::use_cli_compatible_tags`), tags must also be
  prefixed with a timestamp, following: `[0-9]{14}_[a-z0-9-]+`.
  See the [embedded_cli_compatible](https://github.com/jaemk/migrant_lib/blob/master/examples/embedded_cli_compatible.rs)
  example.
- Function migrations must have the signature `fn(ConnConfig) -> Result<(), Box<dyn std::error::Error>>`.
  See the [embedded_programmable](https://github.com/jaemk/migrant_lib/blob/master/examples/embedded_programmable.rs)
  example for a working sample of function migrations.
- When working with embedded and function migrations, the respective database feature must be
  enabled (`d-postgres` / `d-sqlite` / `d-mysql`).


```rust,no_run
# extern crate migrant_lib;
# fn run() -> Result<(), Box<dyn std::error::Error>> {
# let mut config = migrant_lib::Config::from_settings_file("path")?;
fn up(_: migrant_lib::ConnConfig) -> Result<(), Box<dyn std::error::Error>> {
    print!(" Up!");
    Ok(())
}

fn down(_: migrant_lib::ConnConfig) -> Result<(), Box<dyn std::error::Error>> {
    print!(" Down!");
    Ok(())
}

# #[cfg(any(feature="d-sqlite", feature="d-postgres", feature="d-mysql"))]
config.use_migrations(&[
    migrant_lib::FileMigration::with_tag("create-users-table")
        .up("migrations/embedded/create_users_table/up.sql")?
        .down("migrations/embedded/create_users_table/down.sql")?
        .boxed(),
    migrant_lib::EmbeddedMigration::with_tag("create-places-table")
        .up(include_str!("../migrations/embedded/create_places_table/up.sql"))
        .down(include_str!("../migrations/embedded/create_places_table/down.sql"))
        .boxed(),
    migrant_lib::FnMigration::with_tag("custom")
        .up(up)
        .down(down)
        .boxed(),
])?;
# Ok(())
# }
# fn main() { run().unwrap(); }
```


## CLI Compatibility

Migration management identical to the [`migrant`](https://github.com/jaemk/migrant) CLI tool can also be embedded.
This method only supports file-based migrations (so `FileMigration`s or `EmbeddedMigration`s using `include_str!`)
and those migration files names must be timestamped with the format `[0-9]{14}_[a-z0-9-]+`,
Properly named files can be generated by `migrant_lib::new` or the `migrant` CLI tool.
This is required because migration order is implied by file names which must follow
a specific format and contain a valid timestamp.

See the [migrant_cli_compatible](https://github.com/jaemk/migrant_lib/blob/master/examples/migrant_cli_compatible.rs)
example for a working sample where migration files and a `Migrant.toml` config file are available at runtime.

See the [embedded_cli_compatible](https://github.com/jaemk/migrant_lib/blob/master/examples/embedded_cli_compatible.rs)
example for a working sample where the `migrant` CLI tool can be used during development, and database configuration
and migration file contents are embedded in the application.


## Development

See [CONTRIBUTING](https://github.com/jaemk/migrant_lib/blob/master/CONTRIBUTING.md)

----

*/

#![recursion_limit = "1024"]
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate percent_encoding;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate toml;
extern crate url;
extern crate walkdir;

#[cfg(feature = "d-postgres")]
extern crate postgres;

#[cfg(feature = "d-sqlite")]
extern crate rusqlite;

#[cfg(feature = "d-mysql")]
extern crate mysql;

use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{TimeZone, Utc};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use regex::Regex;
use walkdir::WalkDir;

#[macro_use]
mod macros;
pub mod config;
mod connection;
mod drivers;
pub mod errors;
mod migratable;
pub mod migration;

pub use crate::config::{Config, Settings};
pub use crate::connection::ConnConfig;
pub use crate::errors::*;
pub use crate::migratable::Migratable;
pub use crate::migration::{EmbeddedMigration, FileMigration, FnMigration};

static CONFIG_FILE: &str = "Migrant.toml";
static DT_FORMAT: &str = "%Y%m%d%H%M%S";

static SQLITE_CONFIG_TEMPLATE: &str = r#"
# Required, do not edit
database_type = "sqlite"

# Required: Absolute or relative path to your database file.
#           If a relative path is provided, it will be assumed
#           to be relative to this config file dir: `__CONFIG_DIR__/`
# ex.) database_name = "db/db.db"
database_path = "__DB_PATH__"

migration_location = "__MIG_LOC__"  # default "migrations"

"#;

static PG_CONFIG_TEMPLATE: &str = r#"
# Required, do not edit
database_type = "postgres"

# Required database info
database_name = "__DB_NAME__"
database_user = "__DB_USER__"
database_password = "__DB_PASS__"

# Configurable database info
database_host = "__DB_HOST__"         # default "localhost"
database_port = "__DB_PORT__"              # default "5432"
migration_location = "__MIG_LOC__"  # default "migrations"

# Optional customer ssl cert file
# ssl_cert_file = "path/to/certificate.crt.pem.key"

# Extra database connection parameters
# with the format:
# [database_params]
# key = "value"
[database_params]
"#;

static MYSQL_CONFIG_TEMPLATE: &str = r#"
# Required, do not edit
database_type = "mysql"

# Required database info
database_name = "__DB_NAME__"
database_user = "__DB_USER__"
database_password = "__DB_PASS__"

# Configurable database info
database_host = "__DB_HOST__"         # default "localhost"
database_port = "__DB_PORT__"              # default "3306"
migration_location = "__MIG_LOC__"  # default "migrations"

# Extra database connection parameters
# with the format:
# [database_params]
# key = "value"
[database_params]
"#;

lazy_static! {
    // Check if a tag contains any illegal characters
    static ref BAD_TAG_RE: Regex = Regex::new(r"[^a-z0-9-]+").expect("failed to compile regex");

    // For verifying complete stamp+tag names. This is the full timestamped tag used by the cli tool
    static ref FULL_TAG_RE: Regex = Regex::new(r"[0-9]{14}_[a-z0-9-]+").expect("failed to compile regex");

    // For verifying complete tag names that may optionally be prefixed with a timestamp
    static ref FULL_TAG_OPT_STAMP_RE: Regex = Regex::new(r"([0-9]{14}_)?[a-z0-9-]+").expect("failed to compile regex");
}

/// Database type being used
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DbKind {
    Sqlite,
    Postgres,
    MySql,
}
impl std::str::FromStr for DbKind {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "sqlite" => DbKind::Sqlite,
            "postgres" => DbKind::Postgres,
            "mysql" => DbKind::MySql,
            _ => bail_fmt!(ErrorKind::InvalidDbKind, "Invalid Database Kind: {}", s),
        })
    }
}
impl fmt::Display for DbKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DbKind::Postgres => write!(f, "postgres"),
            DbKind::Sqlite => write!(f, "sqlite"),
            DbKind::MySql => write!(f, "mysql"),
        }
    }
}

/// Write the provided bytes to the specified path
fn write_to_path(path: &Path, content: &[u8]) -> Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content)?;
    Ok(())
}

/// Run the given command in the foreground
fn open_file_in_fg(command: &str, file_path: &str) -> Result<()> {
    let mut p = Command::new(command).arg(file_path).spawn()?;
    let ret = p.wait()?;
    if !ret.success() {
        bail_fmt!(
            ErrorKind::ShellCommand,
            "Command `{}` exited with status `{}`",
            command,
            ret
        )
    }
    Ok(())
}

/// Percent encode a string
fn encode(s: &str) -> String {
    percent_encode(s.as_bytes(), NON_ALPHANUMERIC).to_string()
}

/// Prompt the user and return their input
fn prompt(msg: &str) -> Result<String> {
    print!("{}", msg);
    io::stdout().flush()?;
    let mut resp = String::new();
    io::stdin().read_line(&mut resp)?;
    Ok(resp.trim().to_string())
}

#[derive(Debug, Clone)]
/// Represents direction to apply migrations.
/// `Up`   -> up.sql
/// `Down` -> down.sql
pub enum Direction {
    Up,
    Down,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::Direction::*;
        match *self {
            Up => write!(f, "Up"),
            Down => write!(f, "Down"),
        }
    }
}

#[derive(Debug, Clone)]
/// Migration applicator
pub struct Migrator {
    config: Config,
    direction: Direction,
    force: bool,
    fake: bool,
    all: bool,
    show_output: bool,
    swallow_completion: bool,
}

impl Migrator {
    /// Initialize a new `Migrator` with a given `&Config`
    pub fn with_config(config: &Config) -> Self {
        Self {
            config: config.clone(),
            direction: Direction::Up,
            force: false,
            fake: false,
            all: false,
            show_output: true,
            swallow_completion: false,
        }
    }

    /// Set `direction`. Default is `Up`.
    /// `Up`   => run `up.sql`.
    /// `Down` => run `down.sql`.
    pub fn direction(&mut self, dir: Direction) -> &mut Self {
        self.direction = dir;
        self
    }

    /// Set `force` to forcefully apply migrations regardless of errors
    pub fn force(&mut self, force: bool) -> &mut Self {
        self.force = force;
        self
    }

    /// Set `fake` to fake application of migrations.
    /// Applied migrations table will be updated as if migrations were actually run.
    pub fn fake(&mut self, fake: bool) -> &mut Self {
        self.fake = fake;
        self
    }

    /// Set `all` to run all remaining available migrations in the given `direction`
    pub fn all(&mut self, all: bool) -> &mut Self {
        self.all = all;
        self
    }

    /// Toggle migration application output. Default is `true`
    pub fn show_output(&mut self, show_output: bool) -> &mut Self {
        self.show_output = show_output;
        self
    }

    /// Don't return any `ErrorKind::MigrationComplete` errors when running `Migrator::apply`
    ///
    /// All other errors will still be returned
    pub fn swallow_completion(&mut self, swallow_completion: bool) -> &mut Self {
        self.swallow_completion = swallow_completion;
        self
    }

    /// Apply migrations using current configuration
    ///
    /// Returns an `ErrorKind::MigrationComplete` if all migrations in the given
    /// direction have already been applied, unless `swallow_completion` is set to `true`.
    pub fn apply(&self) -> Result<()> {
        let res = self.apply_migration(&self.config);
        if self.swallow_completion {
            match res {
                Ok(_) => (),
                Err(ref e) if e.is_migration_complete() => (),
                Err(e) => return Err(e),
            };
            Ok(())
        } else {
            res
        }
    }

    /// Return the next available up or down migration
    fn next_available<'a>(
        direction: &Direction,
        available: &'a [Box<dyn Migratable>],
        applied: &[String],
    ) -> Result<Option<&'a Box<dyn Migratable>>> {
        Ok(match *direction {
            Direction::Up => {
                for mig in available {
                    let tag = mig.tag();
                    if !applied.contains(&tag) {
                        return Ok(Some(mig));
                    }
                }
                None
            }
            Direction::Down => match applied.last() {
                Some(tag) => {
                    let mig = available.iter().rev().find(|m| &m.tag() == tag);
                    match mig {
                        None => bail_fmt!(ErrorKind::MigrationNotFound, "Tag not found: {}", tag),
                        Some(mig) => Some(mig),
                    }
                }
                None => None,
            },
        })
    }

    /// Apply the migration in the specified direction
    fn run_migration(
        config: &Config,
        direction: &Direction,
        migration: &Box<dyn Migratable>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let db_kind = config.settings.inner.db_kind();
        match *direction {
            Direction::Up => {
                migration.apply_up(db_kind, config)?;
            }
            Direction::Down => {
                migration.apply_down(db_kind, config)?;
            }
        };
        Ok(())
    }

    fn print(&self, s: &str) {
        if self.show_output {
            print_flush!("{}", s);
        }
    }

    fn println(&self, s: &str) {
        if self.show_output {
            println!("{}", s);
        }
    }

    /// Try applying the next available migration in the specified `Direction`
    fn apply_migration(&self, config: &Config) -> Result<()> {
        let migrations = match config.migrations {
            Some(ref migrations) => migrations.clone(),
            None => {
                let mig_dir = config.migration_location()?;
                search_for_migrations(&mig_dir)?
                    .into_iter()
                    .map(|fm| fm.boxed())
                    .collect()
            }
        };
        match Self::next_available(
            &self.direction,
            migrations.as_slice(),
            config.applied.as_slice(),
        )? {
            None => bail_fmt!(
                ErrorKind::MigrationComplete,
                "No un-applied `{}` migrations found",
                self.direction
            ),
            Some(next) => {
                self.print(&format!(
                    "Applying[{}]: {}",
                    self.direction,
                    next.description(&self.direction)
                ));

                if self.fake {
                    self.println("  ✓ (fake)");
                } else {
                    match Self::run_migration(config, &self.direction, next) {
                        Ok(_) => self.println("  ✓"),
                        Err(ref e) => {
                            self.println("");
                            if self.force {
                                self.println(
                                    &format!(" ** Error ** (Continuing because `--force` flag was specified)\n ** {}", e)
                                    );
                            } else {
                                bail_fmt!(
                                    ErrorKind::Migration,
                                    "Migration was unsucessful...\n{}",
                                    e
                                );
                            }
                        }
                    };
                }

                let mig_tag = next.tag();
                match self.direction {
                    Direction::Up => {
                        config.insert_migration_tag(&mig_tag)?;
                    }
                    Direction::Down => {
                        config.delete_migration_tag(&mig_tag)?;
                    }
                }
            }
        };

        let config = config.reload()?;

        if self.all {
            let res = self.apply_migration(&config);
            match res {
                Ok(_) => (),
                Err(error) => {
                    if !error.is_migration_complete() {
                        return Err(error);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Search for a `Migrant.toml` file in the current and parent directories
pub fn search_for_settings_file<T: AsRef<Path>>(base: T) -> Option<PathBuf> {
    let mut base = base.as_ref().to_owned();
    loop {
        for path in fs::read_dir(&base).unwrap() {
            let path = path.unwrap().path();
            if let Some(file) = path.file_name() {
                if file == CONFIG_FILE {
                    return Some(path.clone());
                }
            }
        }
        if base.parent().is_some() {
            base.pop();
        } else {
            return None;
        }
    }
}

/// Search for available migrations in the given migration directory
///
/// Intended only for use with `FileMigration`s not managed directly in source
/// with `Config::use_migrations`.
fn search_for_migrations(mig_root: &Path) -> Result<Vec<FileMigration>> {
    // collect any .sql files into a Map<`stamp-tag`, Vec<up&down files>>
    let mut files = HashMap::new();
    for dir in WalkDir::new(mig_root) {
        if dir.is_err() {
            break;
        }
        let e = dir.unwrap();
        let path = e.path();
        if let Some(ext) = path.extension() {
            if ext.is_empty() || ext != "sql" {
                continue;
            }
            let parent = path.parent().unwrap();
            let key = format!("{}", parent.display());
            let entry = files.entry(key).or_insert_with(Vec::new);
            entry.push(path.to_path_buf());
        }
    }

    // transform up&down files into a Vec<Migration>
    let mut migrations = vec![];
    for (path, migs) in &files {
        let full_name = PathBuf::from(path);
        let full_name = full_name
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| {
                format_err!(
                    ErrorKind::PathError,
                    "Error extracting file-name from: {:?}",
                    full_name
                )
            })?;
        let mut full_name_iter = full_name.split('_');
        let stamp = full_name_iter.next().ok_or_else(|| {
            format_err!(
                ErrorKind::TagError,
                "Invalid tag format: {:?}, \
                 must follow `<timestamp>_<tag>`",
                full_name
            )
        })?;
        let tag = full_name_iter.next().ok_or_else(|| {
            format_err!(
                ErrorKind::TagError,
                "Invalid tag format: {:?}, \
                 must follow `<timestamp>_<tag>`",
                full_name
            )
        })?;
        let stamp = Utc.datetime_from_str(stamp, DT_FORMAT).chain_err(|| {
            format_err!(
                ErrorKind::TagError,
                "Invalid timestamp format {:?}, on tag: {:?}, must follow `{}`",
                stamp,
                full_name,
                DT_FORMAT
            )
        })?;

        let mut up = None;
        let mut down = None;

        for mig in migs.iter() {
            let up_down = mig.file_stem().and_then(OsStr::to_str).ok_or_else(|| {
                format_err!(
                    ErrorKind::PathError,
                    "Error extracting file-stem from: {:?}",
                    full_name
                )
            })?;
            match up_down {
                "up" => up = Some(mig.clone()),
                "down" => down = Some(mig.clone()),
                _ => unreachable!(),
            };
        }
        if up.is_none() {
            bail_fmt!(
                ErrorKind::MigrationNotFound,
                "Up migration not found for tag: {}",
                tag
            )
        }
        if down.is_none() {
            bail_fmt!(
                ErrorKind::MigrationNotFound,
                "Down migration not found for tag: {}",
                tag
            )
        }
        migrations.push(FileMigration {
            up,
            down,
            tag: tag.to_owned(),
            stamp: Some(stamp),
        });
    }

    // sort by timestamps chronologically
    migrations.sort_by(|a, b| a.stamp.unwrap().cmp(&b.stamp.unwrap()));
    Ok(migrations)
}

/// List the currently applied and available migrations under `migration_location`
pub fn list(config: &Config) -> Result<()> {
    let available = match config.migrations {
        None => {
            let mig_dir = config.migration_location()?;
            let migs = search_for_migrations(&mig_dir)?
                .into_iter()
                .map(|file_mig| file_mig.boxed())
                .collect::<Vec<_>>();
            if migs.is_empty() {
                println!("No migrations found under {:?}", &mig_dir);
            }
            migs
        }
        Some(ref migs) => {
            if migs.is_empty() {
                println!("No migrations specified");
            }
            migs.clone()
        }
    };

    if available.is_empty() {
        return Ok(());
    }
    println!("Current Migration Status:");
    for mig in &available {
        let tagname = mig.tag();
        let x = config.applied.contains(&tagname);
        println!(
            " -> [{x}] {name}",
            x = if x { '✓' } else { ' ' },
            name = tagname
        );
    }
    Ok(())
}

/// Returns true if tag name contains illegal characters
fn invalid_tag(tag: &str) -> bool {
    BAD_TAG_RE.is_match(tag)
}

/// Returns true if full optionally timestamped tag is invalid
fn invalid_optional_stamp_tag(tag: &str) -> bool {
    FULL_TAG_OPT_STAMP_RE.captures_iter(tag).count() != 1
}

/// Return true if the full tag is invalid
fn invalid_full_tag(tag: &str) -> bool {
    !FULL_TAG_RE.is_match(tag)
}

/// Create a new migration with the given tag
///
/// Generated tags will follow the format `{DT-STAMP}_{TAG}`
///
/// Intended only for use when running in "migrant CLI compatibility mode"
/// where migrations (`FileMigration`s) are all files with names following
/// the expected timestamp formatted name.
pub fn new(config: &Config, tag: &str) -> Result<()> {
    if invalid_tag(tag) {
        bail_fmt!(
            ErrorKind::Migration,
            "Invalid tag `{}`. Tags can contain [a-z0-9-]",
            tag
        );
    }
    let now = chrono::Utc::now();
    let dt_string = now.format(DT_FORMAT).to_string();
    let folder = format!("{stamp}_{tag}", stamp = dt_string, tag = tag);

    let mig_dir = config.migration_location()?.join(folder);

    fs::create_dir_all(&mig_dir)?;

    let up = "up.sql";
    let down = "down.sql";
    for mig in &[up, down] {
        let mut p = mig_dir.clone();
        p.push(mig);
        let _ = fs::File::create(&p)?;
    }
    Ok(())
}

/// Open a repl connection to the given `Config` settings
///
/// Note, the respective database shell utility is expected to be available in `$PATH`.
///
/// | Database    |    Utility                  |
/// |-------------|-----------------------------|
/// | `postgres`  | `psql`                      |
/// | `sqlite`    | `sqlite3`                   |
/// | `mysql`     | `mysqlsh` (`mysql-shell`)   |
///
pub fn shell(config: &Config) -> Result<()> {
    match config.settings.inner.db_kind() {
        DbKind::Sqlite => {
            let db_path = config.database_path()?;
            let _ = Command::new("sqlite3")
                .arg(db_path.to_str().unwrap())
                .spawn()
                .chain_err(|| {
                    format_err!(
                        ErrorKind::ShellCommand,
                        "Error running command `sqlite3`. Is it available on your PATH?"
                    )
                })?
                .wait()?;
        }
        DbKind::Postgres => {
            let conn_str = config.connect_string()?;
            Command::new("psql")
                .arg(&conn_str)
                .spawn()
                .chain_err(|| {
                    format_err!(
                        ErrorKind::ShellCommand,
                        "Error running command `psql`. Is it available on your PATH?"
                    )
                })?
                .wait()?;
        }
        DbKind::MySql => {
            let conn_str = config.connect_string()?;
            Command::new("mysqlsh")
                .arg("--sql")
                .arg("--uri")
                .arg(conn_str)
                .spawn()
                .chain_err(|| {
                    format_err!(
                        ErrorKind::ShellCommand,
                        "Error running command `mysqhlsh`. Is it available on your PATH?"
                    )
                })?
                .wait()?;
        }
    };
    Ok(())
}

/// Get user's selection of a set of migrations
fn select_from_matches<'a>(tag: &str, matches: &'a [FileMigration]) -> Result<&'a FileMigration> {
    let min = 1;
    let max = matches.len();
    loop {
        println!("* Migrations matching `{}`:", tag);
        for (row, mig) in matches.iter().enumerate() {
            let dt_string = mig
                .stamp
                .expect("Timestamp missing")
                .format(DT_FORMAT)
                .to_string();
            let info = format!("{stamp}_{tag}", stamp = dt_string, tag = mig.tag);
            println!("    {}) {}", row + 1, info);
        }
        print!("\n Please select a migration [1-{}] >> ", max);
        io::stdout().flush()?;
        let mut s = String::new();
        io::stdin().read_line(&mut s)?;
        let n = match s.trim().parse::<usize>() {
            Err(e) => {
                println!("\nError: {}", e);
                continue;
            }
            Ok(n) => {
                if min <= n && n <= max {
                    n - 1
                } else {
                    println!("\nPlease select a number between 1-{}", max);
                    continue;
                }
            }
        };
        return Ok(&matches[n]);
    }
}

/// Open a migration file containing `tag` in its name
///
/// In the case of ambiguous names, the user will be prompted for a selection.
///
/// Intended only for use with `FileMigration`s that were created by
/// `migrant_lib::new` or `migrant` CLI (migration files with names that
/// follow the expected timestamp format), NOT those managed directly in source
/// with `Config::use_migrations`.
pub fn edit(config: &Config, tag: &str, up_down: &Direction) -> Result<()> {
    let mig_dir = config.migration_location()?;

    let available = search_for_migrations(&mig_dir)?;
    if available.is_empty() {
        println!("No migrations found under {:?}", &mig_dir);
        return Ok(());
    }

    let matches = available
        .into_iter()
        .filter(|m| m.tag.contains(tag))
        .collect::<Vec<_>>();
    let n = matches.len();
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let mig = match n {
        0 => bail_fmt!(ErrorKind::Config, "No migrations found with tag: {}", tag),
        1 => &matches[0],
        _ => {
            println!("* Multiple tags found!");
            select_from_matches(tag, matches.as_slice())?
        }
    };
    let file = match *up_down {
        Direction::Up => mig.up.as_ref().expect("UP migration missing").to_owned(),
        Direction::Down => mig
            .down
            .as_ref()
            .expect("DOWN migration missing")
            .to_owned(),
    };
    let file_path = file.to_str().unwrap();
    let command = format!("{} {}", editor, file_path);
    println!("* Running: `{}`", command);
    let _ = prompt(" -- Press [ENTER] to open now or [CTRL+C] to exit and edit manually")?;
    open_file_in_fg(&editor, file_path)
        .map_err(|e| format_err!(ErrorKind::Migration, "Error editing migrant file: {}", e))?;
    Ok(())
}
