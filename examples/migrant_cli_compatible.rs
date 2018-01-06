/*!
Migrant can be used as a library so you can embed the management of migrations
into your binary and don't need to use a secondary tool in production environments.

The majority of `migrant/src/main.rs` could be copied, or just select functionality.
Run with: `cargo run --example migrant_cli_compatible`
*/
extern crate migrant_lib;

use std::env;
use migrant_lib::Config;


fn run() -> Result<(), migrant_lib::Error> {
    let dir = env::current_dir().unwrap();
    let config = match migrant_lib::search_for_settings_file(&dir) {
        None => {
            Config::init_in(&dir)
                .migration_location("migrations/managed")?
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
        println!("[ERROR] {:?}", e);
    }
}
