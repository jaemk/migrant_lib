use super::*;
/// Postgres database functions using shell commands and db drivers
use std;
use std::path::Path;

#[cfg(feature = "d-postgres")]
use postgres::{Client, NoTls};
#[cfg(feature = "d-postgres")]
use std::io::Read;

#[cfg(not(feature = "d-postgres"))]
use std::process::Command;

#[cfg(not(feature = "d-postgres"))]
fn psql_cmd(conn_str: &str, cmd: &str) -> Result<String> {
    let out = Command::new("psql")
        .arg(conn_str)
        .arg("-v")
        .arg("ON_ERROR_STOP=1")
        .arg("-t") // no headers or footer
        .arg("-A") // un-aligned output
        .arg("-F,") // comma separator
        .arg("-c")
        .arg(cmd)
        .output()
        .chain_err(|| {
            format_err!(
                ErrorKind::ShellCommand,
                "Error running command `psql`. Is it available on your PATH?"
            )
        })?;
    if !out.status.success() {
        let stderr = std::str::from_utf8(&out.stderr)?;
        bail_fmt!(
            ErrorKind::Migration,
            "Error executing statement, stderr: `{}`",
            stderr
        );
    }
    let stdout = String::from_utf8(out.stdout)?;
    Ok(stdout)
}

// --
// Check connection
// --
#[cfg(not(feature = "d-postgres"))]
pub fn can_connect(_: Option<&Path>, conn_str: &str) -> Result<bool> {
    let out = Command::new("psql")
        .arg(conn_str)
        .arg("-c")
        .arg("")
        .output()
        .chain_err(|| "Error running command `psql`. Is it available on your PATH?")?;
    Ok(out.status.success())
}

macro_rules! make_connector {
    ($file:expr) => {{
        let cert = std::fs::read($file)
            .map_err(|e| format_err!(ErrorKind::Migration, "postgres cert file error {}", e))?;
        let cert = native_tls::Certificate::from_pem(&cert)
            .map_err(|e| format_err!(ErrorKind::Migration, "postgres cert load error {}", e))?;
        let connector = native_tls::TlsConnector::builder()
            .add_root_certificate(cert)
            .build()
            .map_err(|e| {
                format_err!(ErrorKind::Migration, "postgres tls-connection error {}", e)
            })?;
        postgres_native_tls::MakeTlsConnector::new(connector)
    }};
}

#[cfg(feature = "d-postgres")]
pub fn can_connect(cert: Option<&Path>, conn_str: &str) -> Result<bool> {
    match cert {
        None => match Client::connect(conn_str, NoTls) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        },
        Some(cert) => match Client::connect(conn_str, make_connector!(cert)) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        },
    }
}

// --
// Check `__migrant_migrations` table exists
// --
#[cfg(not(feature = "d-postgres"))]
pub fn migration_table_exists(_: Option<&Path>, conn_str: &str) -> Result<bool> {
    let stdout = psql_cmd(conn_str, sql::PG_MIGRATION_TABLE_EXISTS)?;
    Ok(stdout.trim() == "t")
}

macro_rules! make_connection {
    ($cert:expr, $conn_str:expr) => {{
        match $cert {
            None => Client::connect($conn_str, NoTls),
            Some(cert) => Client::connect($conn_str, make_connector!(cert)),
        }
    }};
}

#[cfg(feature = "d-postgres")]
pub fn migration_table_exists(cert: Option<&Path>, conn_str: &str) -> Result<bool> {
    let mut conn =
        make_connection!(cert, conn_str).map_err(|e| format_err!(ErrorKind::Migration, "{}", e))?;

    let rows = conn
        .query(sql::PG_MIGRATION_TABLE_EXISTS, &[])
        .map_err(|e| format_err!(ErrorKind::Migration, "{}", e))?;
    let exists: bool = rows.iter().next().unwrap().get(0);
    Ok(exists)
}

// --
// Create `__migrant_migrations` table
// --
#[cfg(not(feature = "d-postgres"))]
pub fn migration_setup(cert: Option<&Path>, conn_str: &str) -> Result<bool> {
    if !migration_table_exists(None, conn_str)? {
        psql_cmd(conn_str, sql::CREATE_TABLE)?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(feature = "d-postgres")]
pub fn migration_setup(cert: Option<&Path>, conn_str: &str) -> Result<bool> {
    if !migration_table_exists(cert, conn_str)? {
        let mut conn = make_connection!(cert, conn_str)
            .map_err(|e| format_err!(ErrorKind::Migration, "{}", e))?;
        conn.execute(sql::CREATE_TABLE, &[])
            .map_err(|e| format_err!(ErrorKind::Migration, "{}", e))?;
        return Ok(true);
    }
    Ok(false)
}

// --
// Select all migrations from `__migrant_migrations` table
// --
#[cfg(not(feature = "d-postgres"))]
pub fn select_migrations(cert: Option<&Path>, conn_str: &str) -> Result<Vec<String>> {
    let stdout = psql_cmd(conn_str, sql::GET_MIGRATIONS)?;
    Ok(stdout.trim().lines().map(String::from).collect())
}

#[cfg(feature = "d-postgres")]
pub fn select_migrations(cert: Option<&Path>, conn_str: &str) -> Result<Vec<String>> {
    let mut conn = make_connection!(cert, conn_str)?;
    let rows = conn.query(sql::GET_MIGRATIONS, &[])?;
    Ok(rows.iter().map(|row| row.get(0)).collect())
}

// --
// Insert migration tag into `__migrant_migrations` table
// --
#[cfg(not(feature = "d-postgres"))]
pub fn insert_migration_tag(cert: Option<&Path>, conn_str: &str, tag: &str) -> Result<()> {
    psql_cmd(conn_str, &sql::PG_ADD_MIGRATION.replace("__VAL__", tag))?;
    Ok(())
}

#[cfg(feature = "d-postgres")]
pub fn insert_migration_tag(cert: Option<&Path>, conn_str: &str, tag: &str) -> Result<()> {
    let mut conn = make_connection!(cert, conn_str)?;
    conn.execute(
        "insert into __migrant_migrations (tag) values ($1)",
        &[&tag],
    )?;
    Ok(())
}

// --
// Delete migration tag from `__migrant_migrations` table
// --
#[cfg(not(feature = "d-postgres"))]
pub fn remove_migration_tag(cert: Option<&Path>, conn_str: &str, tag: &str) -> Result<()> {
    psql_cmd(conn_str, &sql::PG_DELETE_MIGRATION.replace("__VAL__", tag))?;
    Ok(())
}

#[cfg(feature = "d-postgres")]
pub fn remove_migration_tag(cert: Option<&Path>, conn_str: &str, tag: &str) -> Result<()> {
    let mut conn = make_connection!(cert, conn_str)?;
    conn.execute("delete from __migrant_migrations where tag = $1", &[&tag])?;
    Ok(())
}

// --
// Apply migration to database
// --
#[cfg(not(feature = "d-postgres"))]
pub fn run_migration(cert: Option<&Path>, conn_str: &str, filename: &Path) -> Result<()> {
    let filename = filename
        .to_str()
        .ok_or_else(|| format_err!(ErrorKind::PathError, "Invalid file path: {:?}", filename))?;
    let migrate = Command::new("psql")
        .arg(&conn_str)
        .arg("-v")
        .arg("ON_ERROR_STOP=1")
        .arg("-f")
        .arg(filename)
        .output()
        .chain_err(|| {
            format_err!(
                ErrorKind::ShellCommand,
                "Error running command `psql`. Is it available on your PATH?"
            )
        })?;
    if !migrate.status.success() {
        let stderr = std::str::from_utf8(&migrate.stderr)?;
        bail_fmt!(
            ErrorKind::Migration,
            "Error executing statement, stderr: `{}`",
            stderr
        );
    }
    Ok(())
}

#[cfg(feature = "d-postgres")]
pub fn run_migration(cert: Option<&Path>, conn_str: &str, filename: &Path) -> Result<()> {
    let mut file = std::fs::File::open(filename)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let mut conn =
        make_connection!(cert, conn_str).map_err(|e| format_err!(ErrorKind::Migration, "{}", e))?;
    conn.batch_execute(&buf)
        .map_err(|e| format_err!(ErrorKind::Migration, "{}", e))?;
    Ok(())
}

#[cfg(not(feature = "d-postgres"))]
pub fn run_migration_str(
    cert: Option<&Path>,
    _conn_str: &str,
    _stmt: &str,
) -> Result<connection::markers::PostgresFeatureRequired> {
    panic!("\n** Migrant ERROR: `d-postgres` feature required **");
}

#[cfg(feature = "d-postgres")]
pub fn run_migration_str(cert: Option<&Path>, conn_str: &str, stmt: &str) -> Result<()> {
    let mut conn =
        make_connection!(cert, conn_str).map_err(|e| format_err!(ErrorKind::Migration, "{}", e))?;
    conn.batch_execute(stmt)
        .map_err(|e| format_err!(ErrorKind::Migration, "{}", e))?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std;
    macro_rules! _try {
        ($exp:expr) => {
            match $exp {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Caught: {}", e);
                    panic!(e)
                }
            }
        };
    }

    #[test]
    fn postgres() {
        let conn_str = std::env::var("POSTGRES_TEST_CONN_STR")
            .expect("POSTGRES_TEST_CONN_STR env variable required");

        // no table before setup
        assert!(can_connect(None, &conn_str).is_ok());
        let is_setup = _try!(migration_table_exists(None, &conn_str));
        assert_eq!(false, is_setup, "Assert migration table does not exist");

        // setup migration table
        let was_setup = _try!(migration_setup(None, &conn_str));
        assert_eq!(
            true, was_setup,
            "Assert `migration_setup` initializes migration table"
        );
        let was_setup = _try!(migration_setup(None, &conn_str));
        assert_eq!(false, was_setup, "Assert `migration_setup` is idempotent");

        // table exists after setup
        let is_setup = _try!(migration_table_exists(None, &conn_str));
        assert!(is_setup, "Assert migration table exists");

        // insert some tags
        _try!(insert_migration_tag(None, &conn_str, "initial"));
        _try!(insert_migration_tag(None, &conn_str, "alter1"));
        _try!(insert_migration_tag(None, &conn_str, "alter2"));

        // get applied
        let migs = _try!(select_migrations(None, &conn_str));
        assert_eq!(3, migs.len(), "Assert 3 migrations applied");

        // remove some tags
        _try!(remove_migration_tag(None, &conn_str, "alter2"));
        let migs = _try!(select_migrations(None, &conn_str));
        assert_eq!(2, migs.len(), "Assert 2 migrations applied");

        _try!(remove_migration_tag(None, &conn_str, "alter1"));
        _try!(remove_migration_tag(None, &conn_str, "initial"));
        let migs = _try!(select_migrations(None, &conn_str));
        assert_eq!(0, migs.len(), "Assert all migrations removed");
    }
}
