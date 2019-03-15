use super::*;
/// MySQL database functions using shell commands and db drivers
use std;
use std::path::Path;

use std::io::Read;

#[cfg(feature = "d-mysql")]
use mysql::{self, Conn};

#[cfg(not(feature = "d-mysql"))]
use serde;
#[cfg(not(feature = "d-mysql"))]
use serde_json;
#[cfg(not(feature = "d-mysql"))]
use std::io::Write;
#[cfg(not(feature = "d-mysql"))]
use std::process::{Command, Stdio};

#[cfg(not(feature = "d-mysql"))]
mod mysql_output {
    #[derive(Deserialize, Clone)]
    pub struct ShellError {
        pub code: u32,
        pub message: String,
        pub state: String,
        #[serde(rename = "type")]
        pub type_: String,
    }

    #[derive(Deserialize, Clone)]
    pub struct Row<T> {
        pub tag: T,
    }

    #[derive(Deserialize, Clone)]
    pub struct ShellOutput<T> {
        pub rows: Option<Vec<Row<T>>>,
        pub error: Option<ShellError>,
    }
}

#[cfg(not(feature = "d-mysql"))]
use self::mysql_output::*;

#[cfg(not(feature = "d-mysql"))]
fn mysql_cmd<T: serde::de::DeserializeOwned + Clone>(
    conn_str: &str,
    cmd: &str,
) -> Result<ShellOutput<T>> {
    // Or with the regular mysql tool
    // mysql -u root --password=[pass] <db> -e "statement" --skip-column-names --batch
    let mut child = Command::new("mysqlsh")
        .arg("--json=raw")
        .arg("--sql")
        .arg("--classic")
        .arg("--uri")
        .arg(conn_str)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .chain_err(|| {
            format_err!(
                ErrorKind::ShellCommand,
                "Error running command `mysqlsh`. Is it available on your PATH?"
            )
        })?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .chain_err(|| format_err!(ErrorKind::ShellCommand, "Error opening mysqlsh stdin"))?;
        stdin
            .write_all(cmd.as_bytes())
            .chain_err(|| format_err!(ErrorKind::ShellCommand, "Error writing to mysqlsh stdin"))?;
    }

    fn fix_json_output(out: &[u8]) -> Result<String> {
        let mut s = String::from("[");
        for line in String::from_utf8(out.to_vec())?.lines() {
            s.push_str(&line);
            s.push(',');
        }
        let mut s = s.trim_right_matches(",").to_owned();
        s.push(']');
        Ok(s)
    }

    let out = child
        .wait_with_output()
        .chain_err(|| format_err!(ErrorKind::ShellCommand, "Error reading mysqlsh stdout"))?;
    if !out.status.success() {
        let output = fix_json_output(&out.stderr)?;
        let output = serde_json::from_str::<Vec<ShellOutput<T>>>(&output)?;
        for chunk in output.iter() {
            if let Some(ref err) = chunk.error {
                bail_fmt!(
                    ErrorKind::Migration,
                    "Error executing statement. {}[code: {}]: {}",
                    err.type_,
                    err.code,
                    err.message
                );
            }
        }
        bail_fmt!(
            ErrorKind::ShellCommand,
            "Command exited in error with any output: {:?}",
            cmd
        );
    }
    let output = fix_json_output(&out.stdout)?;
    let output = serde_json::from_str::<Vec<ShellOutput<T>>>(&output)?;
    for chunk in output.iter() {
        if chunk.rows.is_some() {
            return Ok(chunk.clone());
        }
    }
    bail_fmt!(
        ErrorKind::ShellCommandNoOutput,
        "No row output received from mysql command: {:?}",
        cmd
    );
}

// --
// Check connection
// --
#[cfg(not(feature = "d-mysql"))]
pub fn can_connect(conn_str: &str) -> Result<bool> {
    match mysql_cmd::<String>(conn_str, "") {
        Err(e) => {
            if e.is_shell_command_no_output() {
                return Ok(true);
            }
            let e = Err(e);
            e.chain_err(|| {
                format!(
                    "Unable to connect to mysql database with conn str: {:?}",
                    conn_str
                )
            })?;
            unreachable!();
        }
        Ok(_) => Ok(true),
    }
}

#[cfg(feature = "d-mysql")]
pub fn can_connect(conn_str: &str) -> Result<bool> {
    Conn::new(conn_str).chain_err(|| {
        format!(
            "Unable to connect to mysql database with conn str: {:?}",
            conn_str
        )
    })?;
    Ok(true)
}

// --
// Check `__migrant_migrations` table exists
// --
#[cfg(not(feature = "d-mysql"))]
pub fn migration_table_exists(conn_str: &str) -> Result<bool> {
    let out = mysql_cmd::<u32>(conn_str, sql::MYSQL_MIGRATION_TABLE_EXISTS)?;
    let rows = out.rows.ok_or_else(|| {
        format_err!(
            ErrorKind::ShellCommand,
            "Incomplete json output. Missing `rows`."
        )
    })?;
    assert!(
        rows.len() == 1,
        "Migration table check: Expected 1 returned row"
    );
    Ok(rows[0].tag == 1)
}

#[cfg(feature = "d-mysql")]
pub fn migration_table_exists(conn_str: &str) -> Result<bool> {
    let mut conn = Conn::new(conn_str).chain_err(|| "Connection Error")?;
    let result = conn.query(sql::MYSQL_MIGRATION_TABLE_EXISTS)?;
    let mut rows = vec![];
    for row in result {
        let (val,): (u32,) = mysql::from_row(row.unwrap());
        rows.push(val);
    }
    assert!(
        rows.len() == 1,
        "Migration table check: Expected 1 returned row"
    );
    Ok(rows[0] == 1)
}

// --
// Create `__migrant_migrations` table
// --
#[cfg(not(feature = "d-mysql"))]
pub fn migration_setup(conn_str: &str) -> Result<bool> {
    if !migration_table_exists(conn_str)? {
        mysql_cmd::<u32>(conn_str, sql::MYSQL_CREATE_TABLE)?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(feature = "d-mysql")]
pub fn migration_setup(conn_str: &str) -> Result<bool> {
    if !migration_table_exists(conn_str)? {
        let mut conn = Conn::new(conn_str).chain_err(|| "Connection Error")?;
        conn.query(sql::MYSQL_CREATE_TABLE)
            .chain_err(|| "Error setting up migration table")?;
        return Ok(true);
    }
    Ok(false)
}

// --
// Select all migrations from `__migrant_migrations` table
// --
#[cfg(not(feature = "d-mysql"))]
pub fn select_migrations(conn_str: &str) -> Result<Vec<String>> {
    let out = mysql_cmd::<String>(conn_str, sql::GET_MIGRATIONS)?;
    let rows = out.rows.ok_or_else(|| {
        format_err!(
            ErrorKind::ShellCommand,
            "Incomplete json output. Missing `rows`."
        )
    })?;
    Ok(rows.iter().map(|r| r.tag.to_string()).collect())
}

#[cfg(feature = "d-mysql")]
pub fn select_migrations(conn_str: &str) -> Result<Vec<String>> {
    let mut conn = Conn::new(conn_str).chain_err(|| "Connection Error")?;
    let rows = conn.query(sql::GET_MIGRATIONS)?;
    let mut res = vec![];
    for row in rows {
        let (tag,): (String,) = mysql::from_row(row.unwrap());
        res.push(tag);
    }
    Ok(res)
}

// --
// Insert migration tag into `__migrant_migrations` table
// --
#[cfg(not(feature = "d-mysql"))]
pub fn insert_migration_tag(conn_str: &str, tag: &str) -> Result<()> {
    mysql_cmd::<u32>(conn_str, &sql::MYSQL_ADD_MIGRATION.replace("__VAL__", tag))?;
    Ok(())
}

#[cfg(feature = "d-mysql")]
pub fn insert_migration_tag(conn_str: &str, tag: &str) -> Result<()> {
    let mut conn = Conn::new(conn_str).chain_err(|| "Connection Error")?;
    conn.prep_exec("insert into __migrant_migrations (tag) values (?)", (tag,))?;
    Ok(())
}

// --
// Delete migration tag from `__migrant_migrations` table
// --
#[cfg(not(feature = "d-mysql"))]
pub fn remove_migration_tag(conn_str: &str, tag: &str) -> Result<()> {
    mysql_cmd::<u32>(
        conn_str,
        &sql::MYSQL_DELETE_MIGRATION.replace("__VAL__", tag),
    )?;
    Ok(())
}

#[cfg(feature = "d-mysql")]
pub fn remove_migration_tag(conn_str: &str, tag: &str) -> Result<()> {
    let mut conn = Conn::new(conn_str).chain_err(|| "Connection Error")?;
    conn.prep_exec("delete from __migrant_migrations where tag = ?", (tag,))?;
    Ok(())
}

// --
// Apply migration to database
// --
#[cfg(not(feature = "d-mysql"))]
pub fn run_migration(conn_str: &str, filename: &Path) -> Result<()> {
    let mut file = std::fs::File::open(filename)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    mysql_cmd::<u32>(conn_str, &buf)?;
    Ok(())
}

#[cfg(feature = "d-mysql")]
pub fn run_migration(conn_str: &str, filename: &Path) -> Result<()> {
    let mut file = std::fs::File::open(filename)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let mut conn = Conn::new(conn_str).chain_err(|| "Connection Error")?;
    conn.query(&buf)
        .map_err(|e| format_err!(ErrorKind::Migration, "{}", e))?;
    Ok(())
}

#[cfg(not(feature = "d-mysql"))]
pub fn run_migration_str(
    _conn_str: &str,
    _stmt: &str,
) -> Result<connection::markers::MySQLFeatureRequired> {
    panic!("\n** Migrant ERROR: `d-mysql` feature required **");
}

#[cfg(feature = "d-mysql")]
pub fn run_migration_str(conn_str: &str, stmt: &str) -> Result<()> {
    let mut conn = Conn::new(conn_str).chain_err(|| "Connection Error")?;
    conn.query(stmt)
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
    fn mysql() {
        let conn_str = std::env::var("MYSQL_TEST_CONN_STR")
            .expect("MYSQL_TEST_CONN_STR env variable required");

        // no table before setup
        assert!(can_connect(&conn_str).is_ok());
        let is_setup = _try!(migration_table_exists(&conn_str));
        assert_eq!(false, is_setup, "Assert migration table does not exist");

        // setup migration table
        let was_setup = _try!(migration_setup(&conn_str));
        assert_eq!(
            true, was_setup,
            "Assert `migration_setup` initializes migration table"
        );
        let was_setup = _try!(migration_setup(&conn_str));
        assert_eq!(false, was_setup, "Assert `migration_setup` is idempotent");

        // table exists after setup
        let is_setup = _try!(migration_table_exists(&conn_str));
        assert!(is_setup, "Assert migration table exists");

        // insert some tags
        _try!(insert_migration_tag(&conn_str, "initial"));
        _try!(insert_migration_tag(&conn_str, "alter1"));
        _try!(insert_migration_tag(&conn_str, "alter2"));

        // get applied
        let migs = _try!(select_migrations(&conn_str));
        assert_eq!(3, migs.len(), "Assert 3 migrations applied");

        // remove some tags
        _try!(remove_migration_tag(&conn_str, "alter2"));
        let migs = _try!(select_migrations(&conn_str));
        assert_eq!(2, migs.len(), "Assert 2 migrations applied");

        _try!(remove_migration_tag(&conn_str, "alter1"));
        _try!(remove_migration_tag(&conn_str, "initial"));
        let migs = _try!(select_migrations(&conn_str));
        assert_eq!(0, migs.len(), "Assert all migrations removed");
    }
}
