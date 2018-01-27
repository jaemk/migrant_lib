///! Database migration connection
use {Config};
use errors::*;

#[cfg(feature="-postgres")]
use postgres;

#[cfg(feature="-sqlite")]
use rusqlite;

#[cfg(feature="-mysql")]
use mysql;


#[allow(dead_code)]
pub mod markers {
    pub struct PostgresqlFeatureRequired;
    pub struct SqliteFeatureRequired;
    pub struct MySQLFeatureRequired;
}
#[allow(unused_imports)]
use self::markers::*;


/// Database connection wrapper
#[allow(dead_code)]
pub struct DbConn<'a> {
    config: &'a Config,
}
impl<'a> DbConn<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    /// Generate a `postgres::Connection`, `-postgres` feature required
    #[cfg(not(feature="-postgres"))]
    pub fn pg_connection(&self) -> Result<PostgresqlFeatureRequired> {
        unimplemented!()
    }

    /// Generate a `postgres::Connection`, `-postgres` feature required
    #[cfg(feature="-postgres")]
    pub fn pg_connection(&self) -> Result<postgres::Connection> {
        let conn_str = self.config.connect_string()?;
        Ok(postgres::Connection::connect(conn_str, postgres::TlsMode::None)?)
    }

    /// Generate a `mysql::Conn`, `-mysql` feature required
    #[cfg(not(feature="-mysql"))]
    pub fn mysql_connection(&self) -> Result<MySQLFeatureRequired> {
        unimplemented!()
    }

    /// Generate a `mysql::Conn`, `-mysql` feature required
    #[cfg(feature="-mysql")]
    pub fn mysql_connection(&self) -> Result<mysql::Conn> {
        let conn_str = self.config.connect_string()?;
        Ok(mysql::Conn::new(conn_str)?)
    }

    /// Generate a `rusqlite::Connection`, `-sqlite` feature required
    #[cfg(not(feature="-sqlite"))]
    pub fn sqlite_connection(&self) -> Result<SqliteFeatureRequired> {
        unimplemented!()
    }

    /// Generate a `rusqlite::Connection`, `-sqlite` feature required
    #[cfg(feature="-sqlite")]
    pub fn sqlite_connection(&self) -> Result<rusqlite::Connection> {
        let db_path = self.config.database_path()?;
        Ok(rusqlite::Connection::open(db_path)?)
    }
}

