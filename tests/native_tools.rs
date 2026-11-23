use spatie_db_dumper::{Bzip2Compressor, Database, DbDumper, Error, GzipCompressor, Sqlite};
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
    time::Duration,
};

fn execute(command: &mut Command, input: Option<&[u8]>) -> Vec<u8> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = input {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    drop(child.stdin.take());
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "native tool failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    result.stdout
}

fn sqlite(path: &std::path::Path, sql: &str) -> Vec<u8> {
    execute(Command::new("sqlite3").arg(path).arg(sql), None)
}

#[test]
#[ignore = "requires sqlite3, gzip and bzip2"]
fn sqlite_roundtrip_selection_compression_and_source_protection() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source $(not a shell).sqlite");
    sqlite(
        &source,
        "PRAGMA journal_mode=WAL; CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT); INSERT INTO users VALUES(7,'Alice'),(23,'Björk'); CREATE TABLE logs(message TEXT); INSERT INTO logs VALUES('private log'); CREATE TABLE \"two spaces\"(value TEXT); INSERT INTO \"two spaces\" VALUES('spaced');",
    );
    let dumper = Sqlite::create().set_db_name(source.to_str().unwrap());
    assert_eq!(
        dumper.get_db_tables().unwrap(),
        ["users", "logs", "two spaces"]
    );
    let dump = dir.path().join("dump.sql");
    dumper.dump_to_file(&dump).unwrap();
    let restored = dir.path().join("restored.sqlite");
    execute(
        Command::new("sqlite3").arg(&restored),
        Some(&fs::read(&dump).unwrap()),
    );
    assert_eq!(
        sqlite(&restored, "SELECT id,name FROM users ORDER BY id;"),
        "7|Alice\n23|Björk\n".as_bytes()
    );
    for (selected, expected) in [
        (
            dumper.clone().include_tables("users, two spaces"),
            vec!["users", "two spaces"],
        ),
        (
            dumper.clone().exclude_tables(["logs"]),
            vec!["users", "two spaces"],
        ),
        (
            dumper
                .clone()
                .exclude_tables(["users", "logs", "two spaces"]),
            vec![],
        ),
    ] {
        selected.dump_to_file(&dump).unwrap();
        let restored = tempfile::NamedTempFile::new().unwrap();
        execute(
            Command::new("sqlite3").arg(restored.path()),
            Some(&fs::read(&dump).unwrap()),
        );
        let tables = Sqlite::create()
            .set_db_name(restored.path().to_str().unwrap())
            .get_db_tables()
            .unwrap();
        assert_eq!(tables, expected);
    }
    for (compressed, binary) in [
        (dumper.clone().use_compressor(GzipCompressor), "gzip"),
        (dumper.clone().use_compressor(Bzip2Compressor), "bzip2"),
    ] {
        compressed.dump_to_file(&dump).unwrap();
        let sql = execute(Command::new(binary).arg("-dc").arg(&dump), None);
        assert!(String::from_utf8_lossy(&sql).contains("Björk"));
    }
    assert!(matches!(
        dumper.dump_to_file(&source),
        Err(Error::InvalidConfiguration(_))
    ));
    let alias = dir.path().join("hardlink.sqlite");
    fs::hard_link(&source, &alias).unwrap();
    assert!(dumper.dump_to_file(&alias).is_err());
    assert_eq!(sqlite(&source, "SELECT count(*) FROM users;"), b"2\n");
    let missing = dir.path().join("missing.sqlite");
    assert!(
        Sqlite::create()
            .set_db_name(missing.to_str().unwrap())
            .dump_to_file(&dump)
            .is_err()
    );
    assert!(!missing.exists());
}

#[test]
#[ignore = "requires sqlite3"]
fn sqlite_wal_snapshot_allows_writes_without_including_later_rows() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("wal.sqlite");
    sqlite(
        &source,
        "PRAGMA journal_mode=WAL; CREATE TABLE items(value TEXT); WITH RECURSIVE n(i) AS(SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i<10000) INSERT INTO items SELECT printf('%08d',i) || hex(randomblob(128)) FROM n;",
    );
    let prepared = Sqlite::create()
        .set_db_name(source.to_str().unwrap())
        .get_dump_command()
        .unwrap();
    let mut child = Command::new(prepared.program())
        .args(prepared.args())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(prepared.stdin())
        .unwrap();
    use std::io::Read;
    let mut stdout = child.stdout.take().unwrap();
    let mut first = [0];
    stdout.read_exact(&mut first).unwrap();
    sqlite(
        &source,
        "PRAGMA busy_timeout=0; INSERT INTO items VALUES('written during dump');",
    );
    let mut bytes = vec![first[0]];
    stdout.read_to_end(&mut bytes).unwrap();
    assert!(child.wait().unwrap().success());
    let sql = String::from_utf8(bytes).unwrap();
    assert_eq!(sql.matches("INSERT INTO").count(), 10000);
    assert!(!sql.contains("written during dump"));
    assert_eq!(sqlite(&source, "SELECT count(*) FROM items;"), b"10001\n");
}

fn mysql_client(url: &url::Url, database: bool, client: &str) -> Command {
    let mut command = Command::new(client);
    command
        .arg("--host")
        .arg(url.host_str().unwrap())
        .arg("--port")
        .arg(url.port().unwrap_or(3306).to_string())
        .arg("--user")
        .arg(url.username())
        .env("MYSQL_PWD", url.password().unwrap_or(""));
    if database {
        command.arg(url.path().trim_matches('/'));
    }
    command
}

fn mysql_roundtrip(database: Database, variable: &str, client: &str) {
    let value = std::env::var(variable).expect("set a URL for a disposable test database");
    let url = url::Url::parse(&value).unwrap();
    let name = url.path().trim_matches('/');
    assert!(name.starts_with("db_dumper_test"));
    execute(
        &mut mysql_client(&url, false, client),
        Some(format!("CREATE DATABASE IF NOT EXISTS `{name}`;").as_bytes()),
    );
    execute(&mut mysql_client(&url, true, client), Some(b"DROP TABLE IF EXISTS things,logs; CREATE TABLE things(id INTEGER PRIMARY KEY AUTO_INCREMENT, name VARCHAR(80)); INSERT INTO things VALUES(7,'Alice'),(23,' AUTO_INCREMENT=99'); CREATE TABLE logs(message TEXT); INSERT INTO logs VALUES('private log'); CREATE TRIGGER log_insert AFTER INSERT ON things FOR EACH ROW INSERT INTO logs VALUES('trigger fired');"));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dump.sql");
    let mut dumper = DbDumper::new(database)
        .set_database_url(value)
        .unwrap()
        .use_single_transaction()
        .skip_auto_increment()
        .exclude_tables_data(["logs"])
        .set_timeout(Duration::from_secs(30));
    if let Ok(binary_path) = std::env::var(format!("{variable}_BINARY_PATH")) {
        dumper = dumper.set_dump_binary_path(binary_path);
    }
    if database == Database::MariaDb {
        dumper = dumper.without_sandbox_mode();
    }
    dumper.dump_to_file(&path).unwrap();
    let sql = fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&sql);
    assert!(!text.contains("AUTO_INCREMENT=24"));
    assert!(text.contains(" AUTO_INCREMENT=99"));
    assert!(!text.contains("enable the sandbox mode"));
    execute(
        &mut mysql_client(&url, true, client),
        Some(b"DROP TABLE things,logs;"),
    );
    execute(&mut mysql_client(&url, true, client), Some(&sql));
    assert_eq!(
        execute(
            &mut {
                let mut c = mysql_client(&url, true, client);
                c.args([
                    "--batch",
                    "--skip-column-names",
                    "-e",
                    "SELECT id,name FROM things ORDER BY id;",
                ]);
                c
            },
            None
        ),
        b"7\tAlice\n23\t AUTO_INCREMENT=99\n"
    );
    assert_eq!(
        execute(
            &mut {
                let mut c = mysql_client(&url, true, client);
                c.args([
                    "--batch",
                    "--skip-column-names",
                    "-e",
                    "SELECT count(*) FROM logs;",
                ]);
                c
            },
            None
        ),
        b"0\n"
    );
    let mut check_trigger = mysql_client(&url, true, client);
    check_trigger.args([
        "--batch",
        "--skip-column-names",
        "-e",
        "INSERT INTO things VALUES(24,'later'); SELECT count(*) FROM logs;",
    ]);
    assert_eq!(execute(&mut check_trigger, None), b"1\n");
    execute(
        &mut mysql_client(&url, false, client),
        Some(format!("DROP DATABASE `{name}`;").as_bytes()),
    );
}

#[test]
#[ignore = "requires MySQL and DB_DUMPER_MYSQL_URL"]
fn mysql_native_roundtrip() {
    mysql_roundtrip(Database::MySql, "DB_DUMPER_MYSQL_URL", "mysql");
}

#[test]
#[ignore = "requires MariaDB and DB_DUMPER_MARIADB_URL"]
fn mariadb_native_roundtrip() {
    mysql_roundtrip(Database::MariaDb, "DB_DUMPER_MARIADB_URL", "mariadb");
}

#[test]
#[ignore = "requires PostgreSQL and DB_DUMPER_POSTGRESQL_URL"]
fn postgresql_native_roundtrip() {
    let value = std::env::var("DB_DUMPER_POSTGRESQL_URL")
        .expect("set a URL for a disposable test database");
    let url = url::Url::parse(&value).unwrap();
    assert!(url.path().starts_with("/db_dumper_test"));
    let psql = || {
        let mut c = Command::new("psql");
        c.args(["-X", "-v", "ON_ERROR_STOP=1", &value]);
        c
    };
    execute(&mut psql(), Some(b"DROP TABLE IF EXISTS things,logs; CREATE TABLE things(id INTEGER PRIMARY KEY,name TEXT); INSERT INTO things VALUES(7,'Alice'),(23,'Bjork'); CREATE TABLE logs(message TEXT); INSERT INTO logs VALUES('private log');"));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dump.sql");
    let dumper = DbDumper::new(Database::PostgreSql)
        .set_database_url(&value)
        .unwrap();
    dumper
        .clone()
        .use_inserts()
        .exclude_tables(["logs"])
        .dump_to_file(&path)
        .unwrap();
    let sql = fs::read(&path).unwrap();
    assert!(!String::from_utf8_lossy(&sql).contains("private log"));
    execute(&mut psql(), Some(b"DROP TABLE things;"));
    execute(&mut psql(), Some(&sql));
    assert_eq!(
        execute(
            psql().args(["-t", "-A", "-c", "SELECT id,name FROM things ORDER BY id;"]),
            None
        ),
        b"7|Alice\n23|Bjork\n"
    );
    dumper.do_not_dump_data().dump_to_file(&path).unwrap();
    assert!(!String::from_utf8_lossy(&fs::read(&path).unwrap()).contains("Bjork"));
    execute(&mut psql(), Some(b"DROP TABLE things,logs;"));
}

#[test]
#[ignore = "requires MongoDB Database Tools and DB_DUMPER_MONGODB_URL"]
fn mongodb_native_archive_roundtrip() {
    let value =
        std::env::var("DB_DUMPER_MONGODB_URL").expect("set a URL for a disposable test database");
    let url = url::Url::parse(&value).unwrap();
    let name = url.path().trim_matches('/');
    assert!(name.starts_with("db_dumper_test"));
    let dir = tempfile::tempdir().unwrap();
    let seed = dir.path().join("things.bson");
    let bson =
        b"\x20\x00\x00\x00\x10_id\x00\x07\x00\x00\x00\x02name\x00\x08\x00\x00\x00fixture\x00\x00";
    let mut bson = bson.to_vec();
    let length = bson.len() as i32;
    bson[..4].copy_from_slice(&length.to_le_bytes());
    fs::write(&seed, &bson).unwrap();
    execute(
        Command::new("mongorestore")
            .args(["--uri", &value, "--drop", "--collection", "things"])
            .arg(&seed),
        None,
    );
    let archive = dir.path().join("dump.archive");
    DbDumper::new(Database::MongoDb)
        .set_database_url(&value)
        .unwrap()
        .set_collection("things")
        .dump_to_file(&archive)
        .unwrap();
    assert!(fs::metadata(&archive).unwrap().len() > bson.len() as u64);
    execute(
        Command::new("mongorestore")
            .args(["--uri", &value, "--drop"])
            .arg(format!("--archive={}", archive.display())),
        None,
    );
    let restored = dir.path().join("restored");
    execute(
        Command::new("mongodump")
            .args(["--uri", &value, "--collection", "things", "--out"])
            .arg(&restored),
        None,
    );
    assert_eq!(
        fs::read(restored.join(name).join("things.bson")).unwrap(),
        bson
    );
}
