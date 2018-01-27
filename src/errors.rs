/*!
Error types
*/

use std;
use toml;
use url;
use chrono;
use serde_json;

#[cfg(feature="d-sqlite")]
use rusqlite;

#[cfg(feature="d-postgres")]
use postgres;

#[cfg(feature="d-mysql")]
use mysql;


error_chain! {
    foreign_links {
        Io(std::io::Error);
        StringUtf8Error(std::string::FromUtf8Error);
        StrUtf8Error(std::str::Utf8Error);
        TomlDe(toml::de::Error);
        TomlSe(toml::ser::Error);
        UrlParse(url::ParseError);
        ChronoParse(chrono::ParseError);
        Json(serde_json::Error);
        Sqlite(rusqlite::Error) #[cfg(feature="d-sqlite")];
        Postgres(postgres::Error) #[cfg(feature="d-postgres")];
        MySql(mysql::Error) #[cfg(feature="d-mysql")];
    }
    errors {
        Config(s: String) {
            description("ConfigError")
            display("ConfigError: {}", s)
        }
        Migration(s: String) {
            description("MigrationError")
            display("MigrationError: {}", s)
        }
        MigrationComplete(s: String) {
            description("MigrationComplete")
            display("MigrationComplete: {}", s)
        }
        MigrationNotFound(s: String) {
            description("MigrationNotFound")
            display("MigrationNotFound: {}", s)
        }
        ShellCommand(s: String) {
            description("ShellCommand")
            display("ShellCommandError: {}", s)
        }
        PathError(s: String) {
            description("PathError")
            display("PathError: {}", s)
        }
        TagError(s: String) {
            description("TagError")
            display("TagError: {}", s)
        }
        InvalidDbKind(s: String) {
            description("InvalidDbKind")
            display("InvalidDbKind: {}", s)
        }
    }
}

impl Error {
    pub fn is_migration_complete(&self) -> bool {
        match *self.kind() {
            ErrorKind::MigrationComplete(_) => true,
            _ => false,
        }
    }
}

