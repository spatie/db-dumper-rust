//! Portable database dump behavior using fixture executables.

use spatie_db_dumper::{
    Bzip2Compressor, Compressor, Database, DbDumper, Error, GzipCompressor, MariaDb, MongoDb,
    MySql, PostgreSql, Sqlite,
};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn tools() -> &'static Path {
    static TOOLS: OnceLock<TempDir> = OnceLock::new();
    TOOLS
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let executable = dir
                .path()
                .join(format!("fixture{}", std::env::consts::EXE_SUFFIX));
            assert!(
                Command::new("rustc")
                    .args(["--edition=2024", "tests/fixtures/tool.rs", "-o"])
                    .arg(&executable)
                    .status()
                    .unwrap()
                    .success()
            );
            for binary in [
                "mysqldump",
                "mariadb-dump",
                "pg_dump",
                "mongodump",
                "sqlite3",
            ] {
                fs::copy(
                    &executable,
                    dir.path()
                        .join(format!("{binary}{}", std::env::consts::EXE_SUFFIX)),
                )
                .unwrap();
            }
            dir
        })
        .path()
}

fn mysql() -> MySql {
    MySql::create()
        .set_db_name("dbname")
        .set_user_name("username")
        .set_password("password")
        .set_dump_binary_path(tools())
}

fn strings(command: &spatie_db_dumper::DumpCommand) -> Vec<String> {
    command
        .args()
        .iter()
        .map(|s| s.to_str().unwrap().to_owned())
        .collect()
}

#[test]
fn all_databases_use_their_native_tools_and_defaults() {
    for (database, binary, port) in [
        (Database::MySql, "mysqldump", "3306"),
        (Database::MariaDb, "mariadb-dump", "3306"),
        (Database::PostgreSql, "pg_dump", "5432"),
        (Database::MongoDb, "mongodump", "27017"),
    ] {
        let dumper = DbDumper::new(database)
            .set_db_name("dbname")
            .set_user_name("username")
            .set_dump_binary_path(tools());
        let command = dumper.get_dump_command().unwrap();
        assert_eq!(command.program(), tools().join(binary));
        assert_eq!(dumper.get_host(), "localhost");
        let args = strings(&command);
        if database == Database::MySql || database == Database::MariaDb {
            let path = args[0].strip_prefix("--defaults-extra-file=").unwrap();
            assert!(
                fs::read_to_string(path)
                    .unwrap()
                    .contains(&format!("port={port}"))
            );
            assert_eq!(
                &args[1..],
                ["--skip-comments", "--extended-insert", "dbname"]
            );
        } else {
            assert!(args.iter().any(|s| s == port));
        }
        let output = tempfile::tempdir().unwrap();
        command
            .dump_to_file(output.path().join("dump.sql"))
            .unwrap();
        assert!(
            fs::read_to_string(output.path().join("dump.sql"))
                .unwrap()
                .contains("CREATE TABLE things")
        );
    }
    let source = tempfile::NamedTempFile::new().unwrap();
    let command = Sqlite::create()
        .set_db_name(source.path().to_str().unwrap())
        .get_dump_command()
        .unwrap();
    assert_eq!(command.stdin(), b"BEGIN;\n.dump\n");
    assert_eq!(strings(&command)[..2], ["-readonly", "--bail"]);
}

#[test]
fn mysql_options_and_after_database_arguments_are_preserved() {
    let command = mysql()
        .do_not_create_tables()
        .do_not_dump_data()
        .do_not_skip_comments()
        .do_not_use_extended_inserts()
        .use_single_transaction()
        .skip_lock_tables()
        .do_not_use_column_statistics()
        .use_quick()
        .include_routines()
        .set_socket("/socket with spaces")
        .exclude_tables("cache, tokens")
        .exclude_tables_data(["logs", "sessions"])
        .set_default_character_set("utf8mb4")
        .set_gtid_purged("OFF")
        .add_extra_option("--xml")
        .add_extra_option_after_db_name("--where=id > 7")
        .get_dump_command()
        .unwrap();
    assert_eq!(
        &strings(&command)[1..],
        [
            "--no-create-info",
            "--no-data",
            "--skip-extended-insert",
            "--single-transaction",
            "--skip-lock-tables",
            "--column-statistics=0",
            "--quick",
            "--routines",
            "--socket=/socket with spaces",
            "--ignore-table=dbname.cache",
            "--ignore-table=dbname.tokens",
            "--default-character-set=utf8mb4",
            "--xml",
            "--set-gtid-purged=OFF",
            "dbname",
            "--where=id > 7"
        ]
    );
    let restored = mysql()
        .do_not_skip_comments()
        .skip_comments()
        .do_not_use_extended_inserts()
        .use_extended_inserts()
        .use_single_transaction()
        .do_not_use_single_transaction()
        .skip_lock_tables()
        .do_not_skip_lock_tables()
        .use_quick()
        .do_not_use_quick()
        .skip_auto_increment()
        .do_not_skip_auto_increment()
        .get_dump_command()
        .unwrap();
    assert_eq!(
        &strings(&restored)[1..],
        ["--skip-comments", "--extended-insert", "dbname"]
    );
    let included = mysql()
        .include_tables(["one", "two space"])
        .get_dump_command()
        .unwrap();
    assert_eq!(
        &strings(&included)[3..],
        ["dbname", "--tables", "one", "two space"]
    );
}

#[test]
fn database_options_override_the_database_without_substring_matching() {
    let all = MySql::create()
        .set_user_name("user")
        .add_extra_option("--all-databases")
        .get_dump_command()
        .unwrap();
    assert_eq!(
        &strings(&all)[1..],
        ["--skip-comments", "--extended-insert", "--all-databases"]
    );
    let multiple = mysql()
        .add_extra_option("--databases")
        .add_extra_option("other")
        .exclude_tables(["cache"])
        .get_dump_command()
        .unwrap();
    assert_eq!(
        &strings(&multiple)[1..],
        [
            "--skip-comments",
            "--extended-insert",
            "--ignore-table=other.cache",
            "--databases",
            "other"
        ]
    );
    assert!(
        MySql::create()
            .set_user_name("user")
            .add_extra_option("--where=--all-databases")
            .get_dump_command()
            .is_err()
    );
    assert!(
        mysql()
            .add_extra_option("--databases")
            .get_dump_command()
            .is_err()
    );
}

#[test]
fn credentials_are_escaped_private_and_owned_by_the_prepared_command() {
    let command = mysql()
        .set_password("p\"'\\\n\r\t#;$(touch /never)")
        .set_skip_ssl(true)
        .get_dump_command()
        .unwrap();
    let args = strings(&command);
    let file = PathBuf::from(args[0].strip_prefix("--defaults-extra-file=").unwrap());
    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(
        content,
        "[client]\nuser=\"username\"\npassword=\"p\\\"'\\\\\\n\\r\\t#;$(touch /never)\"\nport=3306\nhost=\"localhost\"\nssl-mode=DISABLED\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert!(!args.iter().any(|s| s.contains("password")));
    drop(command);
    assert!(!file.exists());
    let maria = MariaDb::create()
        .set_db_name("db")
        .set_user_name("user")
        .set_skip_ssl(true)
        .get_dump_command()
        .unwrap();
    assert!(
        fs::read_to_string(
            strings(&maria)[0]
                .strip_prefix("--defaults-extra-file=")
                .unwrap()
        )
        .unwrap()
        .ends_with("skip-ssl\n")
    );
    let custom = mysql()
        .set_skip_ssl(true)
        .set_ssl_flag("ssl-mode=PREFERRED")
        .get_dump_command()
        .unwrap();
    assert!(
        fs::read_to_string(
            strings(&custom)[0]
                .strip_prefix("--defaults-extra-file=")
                .unwrap()
        )
        .unwrap()
        .ends_with("ssl-mode=PREFERRED\n")
    );
}

#[test]
fn postgresql_and_mongodb_options_and_credentials() {
    let pg = PostgreSql::create()
        .set_database_url("postgres://u%3Aser:p%5Ca%3Ass@host:5439/db?host=other&database=final")
        .unwrap()
        .set_socket("/tmp/pg")
        .use_inserts()
        .do_not_create_tables()
        .do_not_dump_data()
        .include_tables(["one", "two"])
        .add_extra_option("--no-owner")
        .get_dump_command()
        .unwrap();
    assert_eq!(
        strings(&pg),
        [
            "-w",
            "-U",
            "u:ser",
            "-h",
            "/tmp/pg",
            "-p",
            "5439",
            "--inserts",
            "--data-only",
            "--schema-only",
            "--no-owner",
            "-t",
            "one",
            "-t",
            "two"
        ]
    );
    let pgpass = &pg
        .environment()
        .iter()
        .find(|(key, _)| key == "PGPASSFILE")
        .unwrap()
        .1;
    assert_eq!(
        fs::read_to_string(pgpass).unwrap(),
        "/tmp/pg:5439:final:u\\:ser:p\\\\a\\:ss\n"
    );
    let excluded = PostgreSql::create()
        .set_db_name("db")
        .set_user_name("u")
        .exclude_tables(["logs", "cache"])
        .get_dump_command()
        .unwrap();
    assert_eq!(&strings(&excluded)[7..], ["-T", "logs", "-T", "cache"]);
    let mongo = MongoDb::create()
        .set_db_name("db")
        .set_user_name("u")
        .set_password("p\"\\\n")
        .set_host("host")
        .set_port(27018)
        .set_collection("events")
        .set_authentication_database("admin")
        .get_dump_command()
        .unwrap();
    let args = strings(&mongo);
    assert_eq!(
        &args[..6],
        ["--db", "db", "--archive", "--username", "u", "--config"]
    );
    assert_eq!(
        &args[7..],
        [
            "--host",
            "host",
            "--port",
            "27018",
            "--collection",
            "events",
            "--authenticationDatabase",
            "admin"
        ]
    );
    assert_eq!(
        fs::read_to_string(&args[6]).unwrap(),
        "password: \"p\\\"\\\\\\n\"\n"
    );
}

#[test]
fn urls_preserve_zero_decode_components_and_override_only_supplied_settings() {
    let dumper = mysql()
        .set_database_url("mysql://u:p@h:3307/other?database=0&password=0&username=zero")
        .unwrap();
    assert_eq!(dumper.get_db_name(), "0");
    assert_eq!(dumper.get_host(), "h");
    assert_eq!(
        dumper.get_database_url(),
        "mysql://u:p@h:3307/other?database=0&password=0&username=zero"
    );
    let credentials = dumper.get_dump_command().unwrap();
    assert!(
        fs::read_to_string(
            strings(&credentials)[0]
                .strip_prefix("--defaults-extra-file=")
                .unwrap()
        )
        .unwrap()
        .contains("password=\"0\"")
    );
    assert!(MySql::create().set_database_url("://bad").is_err());
    assert!(
        mysql()
            .set_database_url("mysql://host/db?port=bad")
            .is_err()
    );
    for (url, expected) in [
        ("sqlite:///path/to/db.sqlite", "/path/to/db.sqlite"),
        ("sqlite:relative.sqlite", "relative.sqlite"),
        ("sqlite3:///with%20space.sqlite", "/with space.sqlite"),
    ] {
        assert_eq!(
            Sqlite::create()
                .set_database_url(url)
                .unwrap()
                .get_db_name(),
            expected
        );
    }
}

#[test]
fn conflicting_and_incomplete_configuration_is_rejected() {
    assert!(MySql::create().get_dump_command().is_err());
    assert!(PostgreSql::create().get_dump_command().is_err());
    assert!(MongoDb::create().get_dump_command().is_err());
    for dumper in [
        mysql().include_tables(["one"]).exclude_tables(["two"]),
        mysql().exclude_tables(["two"]).include_tables(["one"]),
        mysql().use_append_mode().use_compressor(GzipCompressor),
        mysql().use_compressor(GzipCompressor).use_append_mode(),
        mysql().set_ssl_flag("bad"),
        mysql().set_db_name("--xml"),
        mysql().include_tables(["--xml"]),
        mysql().set_password("password\0suffix"),
    ] {
        assert!(matches!(
            dumper.get_dump_command(),
            Err(Error::InvalidConfiguration(_))
        ));
    }
    assert!(
        PostgreSql::create()
            .set_db_name("db")
            .set_user_name("user\ninjected")
            .get_dump_command()
            .is_err()
    );
    assert_eq!(
        mysql()
            .use_compressor(GzipCompressor)
            .get_compressor_extension(),
        "gz"
    );
    assert_eq!(
        mysql()
            .use_compressor(Bzip2Compressor)
            .get_compressor_extension(),
        "bz2"
    );
    assert_eq!(mysql().get_compressor_extension(), "");
}

#[test]
fn literal_arguments_and_paths_never_invoke_a_shell() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("injected");
    let option = format!(
        "--where=$(touch '{}'); echo > '{}'",
        marker.display(),
        marker.display()
    );
    let target = dir.path().join("dump $(echo unsafe) [space].sql");
    mysql()
        .add_extra_option("--fixture-mode=echo")
        .add_extra_option(&option)
        .dump_to_file(&target)
        .unwrap();
    assert!(
        fs::read_to_string(&target)
            .unwrap()
            .split('\0')
            .any(|s| s == option)
    );
    assert!(!marker.exists());
}

struct TestCompressor(&'static str);
impl Compressor for TestCompressor {
    fn program(&self) -> PathBuf {
        tools().join(format!("fixture{}", std::env::consts::EXE_SUFFIX))
    }
    fn args(&self) -> Vec<OsString> {
        vec![format!("--fixture-mode={}", self.0).into()]
    }
    fn use_extension(&self) -> &str {
        "test"
    }
}

#[test]
fn failures_timeouts_empty_output_and_compressor_failures_preserve_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("dump.sql");
    fs::write(&target, "original").unwrap();
    assert!(matches!(
        mysql()
            .add_extra_option("--fixture-mode=fail")
            .dump_to_file(&target),
        Err(Error::ProcessFailed { code: Some(7), .. })
    ));
    assert!(matches!(
        mysql()
            .add_extra_option("--fixture-mode=empty")
            .use_compressor(TestCompressor("copy"))
            .dump_to_file(&target),
        Err(Error::EmptyDump)
    ));
    assert!(matches!(
        mysql()
            .use_compressor(TestCompressor("fail"))
            .dump_to_file(&target),
        Err(Error::ProcessFailed { code: Some(7), .. })
    ));
    let start = Instant::now();
    assert!(matches!(
        mysql()
            .add_extra_option("--fixture-mode=sleep")
            .set_timeout(Duration::from_millis(50))
            .dump_to_file(&target),
        Err(Error::Timeout)
    ));
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(matches!(
        mysql()
            .use_compressor(TestCompressor("sleep"))
            .set_timeout(Duration::from_millis(50))
            .dump_to_file(&target),
        Err(Error::Timeout)
    ));
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    assert!(matches!(
        mysql()
            .set_dump_binary_path(dir.path())
            .dump_to_file(&target),
        Err(Error::Io(_))
    ));
}

#[test]
fn noisy_processes_and_custom_compression_complete_without_pipe_deadlocks() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("dump.sql");
    mysql()
        .add_extra_option("--fixture-mode=noisy")
        .use_compressor(TestCompressor("copy"))
        .dump_to_file(&target)
        .unwrap();
    assert_eq!(fs::read(&target).unwrap(), vec![b'x'; 1024 * 1024]);
}

#[test]
fn append_and_output_filters_keep_data_and_remove_only_the_requested_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("dump.sql");
    fs::write(&target, "previous\n").unwrap();
    MariaDb::create()
        .set_db_name("db")
        .set_user_name("u")
        .set_dump_binary_path(tools())
        .without_sandbox_mode()
        .skip_auto_increment()
        .use_append_mode()
        .dump_to_file(&target)
        .unwrap();
    let output = fs::read_to_string(&target).unwrap();
    assert_eq!(
        output,
        "previous\nCREATE TABLE things (id INT);\n) ENGINE=InnoDB;\nINSERT INTO things VALUES (' AUTO_INCREMENT=99');\n"
    );
}

#[test]
fn mysql_data_exclusions_preserve_schema_and_fail_as_one_dump() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("dump.sql");
    fs::write(&target, "original").unwrap();
    let dumper = mysql().exclude_tables_data(["logs"]);
    assert!(matches!(
        dumper
            .clone()
            .add_extra_option("--fixture-mode=fail-data")
            .dump_to_file(&target),
        Err(Error::ProcessFailed { code: Some(8), .. })
    ));
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    assert!(
        dumper
            .clone()
            .add_extra_option("--xml")
            .get_dump_command()
            .is_err()
    );
    assert!(
        dumper
            .clone()
            .do_not_dump_data()
            .get_dump_command()
            .unwrap()
            .additional_args()
            .is_empty()
    );
    let data_only = dumper.do_not_create_tables().get_dump_command().unwrap();
    assert!(strings(&data_only).contains(&"--ignore-table=dbname.logs".into()));
    assert!(data_only.additional_args().is_empty());
    let maria = MariaDb::create()
        .set_db_name("db")
        .set_user_name("u")
        .exclude_tables_data(["logs"])
        .get_dump_command()
        .unwrap();
    assert!(strings(&maria).contains(&"--ignore-table-data=db.logs".into()));
    assert!(maria.additional_args().is_empty());
}
