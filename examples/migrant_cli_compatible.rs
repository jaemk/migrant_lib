/*!
The functionality of the Migrant CLI tool can be used as a library so you can embed general
database & migration management in your binary and don't need to use a secondary tool
in production environments.

Select functionality can be copied from https://github.com/jaemk/migrant/blob/master/src/main.rs

Run with: `cargo run --example migrant_cli_compatible [--features d-sqlite]`
Note: Running without features will use the corresponding database shell commands.
      Use the respective `--features` to use the actual database driver libraries (`d-sqlite`, `d-postgres`, `d-mysql`)
*/
extern crate migrant_lib;

use std::env;
use migrant_lib::Config;
use migrant_lib::config::SqliteSettingsBuilder;
// use migrant_lib::config::PostgresSettingsBuilder;
// use migrant_lib::config::MySqlSettingsBuilder;


fn run() -> Result<(), migrant_lib::Error> {
    let dir = env::current_dir().unwrap();
    let config = match migrant_lib::search_for_settings_file(&dir) {
        None => {
            Config::init_in(&dir)
                .with_sqlite_options(
                    SqliteSettingsBuilder::empty()
                        .database_path("db/db.db")?
                        .migration_location("migrations/managed")?)
                // .with_postgres_options(
                //     PostgresSettingsBuilder::empty()
                //         .database_name("testing")
                //         .database_user("testing")
                //         .database_password("testing")
                //         .database_host("localhost")
                //         .database_port(5432)
                //         .database_params(&[("port", "5432"), ("sslmode", "disable")])
                //         .migration_location("migrations/managed")?)
                // .with_mysql_options(
                //     MySqlSettingsBuilder::empty()
                //         .database_name("testing")
                //         .database_user("testing")
                //         .database_password("pass")
                //         .database_host("localhost")
                //         .database_port(3306)
                //         .database_params(&[("prefer_socket", "true")])
                //         .migration_location("migrations/managed")?)
                .initialize()?;
            println!("\nSettings file and migrations table initialized. \
                      Please run again to apply migrations.");
            return Ok(())
        }
        Some(p) => Config::from_settings_file(&p)?
    };
    config.reload()?;

    println!("Applying all migrations...");
    migrant_lib::Migrator::with_config(&config)
        .direction(migrant_lib::Direction::Up)
        .all(true)
        .apply()?;
    let config = config.reload()?;
    migrant_lib::list(&config)?;

    println!("Unapplying all migrations...");
    migrant_lib::Migrator::with_config(&config)
        .direction(migrant_lib::Direction::Down)
        .all(true)
        .apply()?;
    let config = config.reload()?;
    migrant_lib::list(&config)?;
    Ok(())
}

pub fn main() {
    if let Err(e) = run() {
        println!("[ERROR] {}", e);
    }
}
