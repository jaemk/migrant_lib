#[cfg(feature = "d-sqlite")]
use crate::config::DbConnection;
use crate::errors::*;
///! Database migration connection
use crate::{Config, DbKind};

#[allow(dead_code)]
pub mod markers {
    #[derive(Clone)]
    pub struct PostgresFeatureRequired;
    #[derive(Clone)]
    pub struct MySQLFeatureRequired;
    #[derive(Clone)]
    pub struct PostgresOrMySQLFeatureRequired;
    #[derive(Clone)]
    pub struct SqliteFeatureRequired;
    #[derive(Clone)]
    pub struct DatabaseFeatureRequired;
}
#[allow(unused_imports)]
use self::markers::*;

/// Database connection information
#[allow(dead_code)]
pub struct ConnConfig<'a> {
    config: &'a Config,
}
impl<'a> ConnConfig<'a> {
    pub(crate) fn new(config: &'a Config) -> Self {
        Self { config }
    }

    /// Return the database type
    pub fn database_type(&self) -> DbKind {
        self.config.database_type()
    }

    /// Return a connection string for postgres or mysql
    #[cfg(not(any(feature = "d-postgres", feature = "d-mysql")))]
    pub fn connect_string(&self) -> Result<PostgresOrMySQLFeatureRequired> {
        unimplemented!()
    }

    /// Return a connection string for postgres or mysql
    #[cfg(any(feature = "d-postgres", feature = "d-mysql"))]
    pub fn connect_string(&self) -> Result<String> {
        self.config.connect_string()
    }

    /// Return a sqlite database path
    #[cfg(not(feature = "d-sqlite"))]
    pub fn database_path(&self) -> Result<SqliteFeatureRequired> {
        unimplemented!()
    }

    /// Return a sqlite database path
    #[cfg(feature = "d-sqlite")]
    pub fn database_path(&self) -> Result<::std::path::PathBuf> {
        self.config.database_path()
    }

    /// Return a rusqlite database connection
    #[cfg(feature = "d-sqlite")]
    pub fn database_connection(&self) -> Result<Option<DbConnection>> {
        self.config.database_connection()
    }
}
