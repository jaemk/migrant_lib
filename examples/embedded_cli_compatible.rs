/*!
This example shows functionality for behaving in a fully compatible manner with the migrant CLI tool,
while still embedding migrations in the application. During development, the CLI tool can be used
to apply migrations from files. When deployed, the application will have the migration file
contents embedded. During development, the CLI tool can use a `Migrant.toml` configuration file,
while the application can embed the settings to avoid configuration in deployed environments.

NOTE: The feature-gates are only required here so the example will compile when running
      tests with and without features. In regular usage, the `cfg`s are not required since
      the specified database feature should be enabled in your `Cargo.toml` entry.

This should be run with `cargo run --example embedded_cli_compatible --features d-sqlite`
*/
extern crate migrant_lib;

#[cfg(feature="d-sqlite")]
use std::env;
#[cfg(feature="d-sqlite")]
use migrant_lib::{Config, Settings, Migrator, Direction, EmbeddedMigration};


#[cfg(feature="d-sqlite")]
fn run() -> Result<(), Box<std::error::Error>> {
    let path = env::current_dir()?;
    let path = path.join("db/embedded_example.db");
    let settings = Settings::configure_sqlite()
        .database_path(&path)?
        .build()?;

    let mut config = Config::with_settings(&settings);

    // Initialize database migrations table
    config.setup()?;

    // Toggle setting so tags are validated in a cli compatible manner.
    // This needs to happen before any call to `Config::use_migrations` or `Config::reload`
    config.use_cli_compatible_tags(true);

    // Define migrations
    config.use_migrations(&[
        EmbeddedMigration::with_tag("20180105040947_initial")
            .up(include_str!("../migrations/managed/20180105040947_initial/up.sql"))
            .down(include_str!("../migrations/managed/20180105040947_initial/down.sql"))
            .boxed(),
        EmbeddedMigration::with_tag("20180105040952_second")
            .up(include_str!("../migrations/managed/20180105040952_second/up.sql"))
            .down(include_str!("../migrations/managed/20180105040952_second/down.sql"))
            .boxed(),
    ])?;

    // Reload config, ping the database for applied migrations
    let config = config.reload()?;

    println!("Applying migrations...");
    Migrator::with_config(&config)
        .all(true)
        .show_output(false)
        .swallow_completion(true)
        .apply()?;

    let config = config.reload()?;
    migrant_lib::list(&config)?;

    println!("\nUnapplying migrations...");
    Migrator::with_config(&config)
        .all(true)
        .direction(Direction::Down)
        .swallow_completion(true)
        .apply()?;

    let config = config.reload()?;
    migrant_lib::list(&config)?;
    Ok(())
}


#[cfg(not(feature="d-sqlite"))]
fn run() -> Result<(), Box<std::error::Error>> {
    Err("d-sqlite database feature required")?;
    Ok(())
}

pub fn main() {
    if let Err(e) = run() {
        println!("[ERROR] {}", e);
    }
}

