use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};

use tempfile::NamedTempFile;
use wait_timeout::ChildExt;

use crate::{Database, Error, dumper::Config};

/// A shell-free prepared dump, owning its temporary credential files.
///
/// `Debug` is deliberately unavailable. Extra options may themselves be secrets.
/// Dropping this value removes its credential files.
#[expect(
    missing_debug_implementations,
    reason = "Arguments may contain credentials"
)]
pub struct DumpCommand {
    program: PathBuf,
    args: Vec<OsString>,
    additional_args: Vec<Vec<OsString>>,
    env: Vec<(OsString, OsString)>,
    input: Vec<u8>,
    credentials: Option<NamedTempFile>,
    config: Config,
}

impl DumpCommand {
    pub(crate) fn prepare(mut config: Config) -> Result<Self, Error> {
        validate(&config)?;
        let binary = match config.database {
            Database::MySql => "mysqldump",
            Database::MariaDb => "mariadb-dump",
            Database::PostgreSql => "pg_dump",
            Database::Sqlite => "sqlite3",
            Database::MongoDb => "mongodump",
        };
        let mut dump = Self {
            program: config.binary_path.join(binary),
            args: vec![],
            additional_args: vec![],
            env: vec![],
            input: vec![],
            credentials: None,
            config: config.clone(),
        };
        let mut args: Vec<OsString> = vec![];
        match config.database {
            Database::MySql | Database::MariaDb => {
                let credentials = mysql_credentials(&config);
                let file = credential_file(credentials.as_bytes())?;
                let mut arg = OsString::from("--defaults-extra-file=");
                arg.push(file.path());
                args.push(arg);
                dump.credentials = Some(file);
                for (enabled, arg) in [
                    (!config.create_tables, "--no-create-info"),
                    (!config.include_data, "--no-data"),
                    (config.skip_comments, "--skip-comments"),
                    (
                        true,
                        if config.extended_inserts {
                            "--extended-insert"
                        } else {
                            "--skip-extended-insert"
                        },
                    ),
                    (config.single_transaction, "--single-transaction"),
                    (config.skip_lock_tables, "--skip-lock-tables"),
                    (config.no_column_statistics, "--column-statistics=0"),
                    (config.quick, "--quick"),
                    (config.routines, "--routines"),
                ] {
                    if enabled {
                        args.push(arg.into());
                    }
                }
                if !config.socket.as_os_str().is_empty() {
                    let mut socket = OsString::from("--socket=");
                    socket.push(&config.socket);
                    args.push(socket);
                }
                let db_name = database_from_options(&config).unwrap_or(&config.db_name);
                for table in &config.exclude {
                    args.push(format!("--ignore-table={db_name}.{table}").into());
                }
                if config.database == Database::MariaDb {
                    for table in &config.exclude_data {
                        args.push(format!("--ignore-table-data={db_name}.{table}").into());
                    }
                }
                if !config.charset.is_empty() {
                    args.push(format!("--default-character-set={}", config.charset).into());
                }
                args.extend(config.extra.iter().cloned());
                if config.gtid_purged != "AUTO" {
                    args.push(format!("--set-gtid-purged={}", config.gtid_purged).into());
                }
                if !has_database_option(&config) {
                    args.push(config.db_name.clone().into());
                }
                if !config.include.is_empty() {
                    args.push("--tables".into());
                    args.extend(config.include.iter().map(Into::into));
                }
                args.extend(config.extra_after.iter().cloned());
                if config.database == Database::MySql
                    && !config.exclude_data.is_empty()
                    && config.include_data
                {
                    let mut data_args = args.clone();
                    for table in &config.exclude_data {
                        data_args.push(format!("--ignore-table={db_name}.{table}").into());
                    }
                    if config.create_tables {
                        let mut trigger_args = args.clone();
                        args.extend(["--no-data", "--skip-triggers"].map(Into::into));
                        data_args.extend(
                            [
                                "--no-create-info",
                                "--skip-triggers",
                                "--skip-routines",
                                "--skip-events",
                                "--set-gtid-purged=OFF",
                            ]
                            .map(Into::into),
                        );
                        trigger_args.extend(
                            [
                                "--no-create-info",
                                "--no-data",
                                "--skip-routines",
                                "--skip-events",
                                "--set-gtid-purged=OFF",
                            ]
                            .map(Into::into),
                        );
                        dump.additional_args = vec![data_args, trigger_args];
                    } else {
                        args = data_args;
                    }
                }
            }
            Database::PostgreSql => {
                let file = credential_file(pg_credentials(&config)?.as_bytes())?;
                dump.env.push(("PGPASSFILE".into(), file.path().into()));
                dump.env
                    .push(("PGDATABASE".into(), config.db_name.clone().into()));
                dump.credentials = Some(file);
                args.extend([
                    "-w".into(),
                    "-U".into(),
                    config.user_name.clone().into(),
                    "-h".into(),
                    if config.socket.as_os_str().is_empty() {
                        config.host.clone().into()
                    } else {
                        config.socket.clone().into_os_string()
                    },
                    "-p".into(),
                    config.port.to_string().into(),
                ]);
                if config.inserts {
                    args.push("--inserts".into());
                }
                if !config.create_tables {
                    args.push("--data-only".into());
                }
                if !config.include_data {
                    args.push("--schema-only".into());
                }
                args.extend(config.extra.iter().cloned());
                for table in &config.include {
                    args.extend(["-t".into(), table.into()]);
                }
                for table in &config.exclude {
                    args.extend(["-T".into(), table.into()]);
                }
            }
            Database::MongoDb => {
                args.extend([
                    "--db".into(),
                    config.db_name.clone().into(),
                    "--archive".into(),
                ]);
                if !config.user_name.is_empty() {
                    args.extend(["--username".into(), config.user_name.clone().into()]);
                }
                if !config.password.is_empty() {
                    let content = format!(
                        "password: {}\n",
                        serde_json::to_string(&config.password).expect("string serialization")
                    );
                    let file = credential_file(content.as_bytes())?;
                    args.extend(["--config".into(), file.path().into()]);
                    dump.credentials = Some(file);
                }
                args.extend([
                    "--host".into(),
                    config.host.clone().into(),
                    "--port".into(),
                    config.port.to_string().into(),
                ]);
                if let Some(collection) = &config.collection {
                    args.extend(["--collection".into(), collection.into()]);
                }
                if let Some(database) = &config.authentication_database {
                    args.extend(["--authenticationDatabase".into(), database.into()]);
                }
            }
            Database::Sqlite => {
                let path = std::fs::canonicalize(&config.db_name)?;
                args.extend(["-readonly".into(), "--bail".into(), path.into_os_string()]);
                let tables = if config.exclude.is_empty() {
                    config.include.clone()
                } else {
                    sqlite_tables(&config)?
                        .into_iter()
                        .filter(|table| !config.exclude.contains(table))
                        .collect()
                };
                let mut input = String::from("BEGIN;\n");
                if config.exclude.is_empty() || !tables.is_empty() {
                    input.push_str(".dump");
                    for table in &tables {
                        if table.contains(['\r', '\n', '\0']) {
                            return Err(invalid(
                                "SQLite table patterns cannot contain line breaks or NUL",
                            ));
                        }
                        input.push(' ');
                        input
                            .push_str(&serde_json::to_string(table).expect("string serialization"));
                    }
                    input.push('\n');
                } else {
                    input.push_str("SELECT 'BEGIN TRANSACTION;';\nSELECT 'COMMIT;';\n");
                }
                dump.input = input.into_bytes();
                config.db_name = std::fs::canonicalize(&config.db_name)?
                    .to_string_lossy()
                    .into_owned();
                dump.config = config;
            }
        }
        dump.args = args;
        Ok(dump)
    }

    /// The executable, without any shell quoting.
    pub fn program(&self) -> &Path {
        &self.program
    }
    /// Literal arguments. User-provided extra arguments may contain secrets.
    pub fn args(&self) -> &[OsString] {
        &self.args
    }
    /// Subsequent MySQL data and trigger passes when excluding tables' data.
    /// Empty for ordinary single-process dumps.
    pub fn additional_args(&self) -> &[Vec<OsString>] {
        &self.additional_args
    }
    /// Environment entries set by this dump. Other environment entries are inherited.
    pub fn environment(&self) -> &[(OsString, OsString)] {
        &self.env
    }
    /// Input supplied to the native tool (SQLite's snapshot and dump commands).
    pub fn stdin(&self) -> &[u8] {
        &self.input
    }

    /// Run this prepared dump. Credential files remain valid until this value is dropped.
    /// Existing output is not replaced when a dump or compressor fails.
    pub fn dump_to_file(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        if self.config.database == Database::Sqlite
            && path.exists()
            && same_file::is_same_file(&self.config.db_name, path)?
        {
            return Err(invalid("SQLite source and dump destination must differ"));
        }
        if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(invalid("dump destination must not be a symlink"));
        }
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let start = Instant::now();
        let mut raw = NamedTempFile::new_in(parent)?;
        let mut input = tempfile::tempfile()?;
        input.write_all(&self.input)?;
        input.rewind()?;
        let mut command = Command::new(&self.program);
        command.args(&self.args).envs(self.env.iter().cloned());
        if self.config.database == Database::PostgreSql {
            command
                .env_remove("PGPASSWORD")
                .env_remove("PGSERVICE")
                .env_remove("PGSERVICEFILE");
        }
        run(&mut command, input, raw.reopen()?, &self.config, start)?;
        for args in &self.additional_args {
            let output = OpenOptions::new().append(true).open(raw.path())?;
            let mut command = Command::new(&self.program);
            command.args(args);
            run(
                &mut command,
                tempfile::tempfile()?,
                output,
                &self.config,
                start,
            )?;
        }
        if raw.as_file().metadata()?.len() == 0 {
            return Err(Error::EmptyDump);
        }
        raw.as_file_mut().rewind()?;
        if self.config.skip_auto_increment
            || (self.config.database == Database::MariaDb && !self.config.sandbox)
        {
            let mut filtered = NamedTempFile::new_in(parent)?;
            filter(raw.as_file_mut(), filtered.as_file_mut(), &self.config)?;
            raw = filtered;
        }
        if raw.as_file().metadata()?.len() == 0 {
            return Err(Error::EmptyDump);
        }
        if let Some(compressor) = &self.config.compressor {
            let compressed = NamedTempFile::new_in(parent)?;
            let mut command = Command::new(compressor.program());
            command.args(compressor.args());
            raw.as_file_mut().rewind()?;
            run(
                &mut command,
                raw.reopen()?,
                compressed.reopen()?,
                &self.config,
                start,
            )?;
            if compressed.as_file().metadata()?.len() == 0 {
                return Err(Error::EmptyDump);
            }
            raw = compressed;
        }
        if self.config.append {
            raw.as_file_mut().rewind()?;
            let mut destination = OpenOptions::new().create(true).append(true).open(path)?;
            std::io::copy(raw.as_file_mut(), &mut destination)?;
            destination.flush()?;
        } else {
            raw.persist(path).map_err(|error| error.error)?;
        }
        Ok(())
    }
}

fn invalid(message: &str) -> Error {
    Error::InvalidConfiguration(message.into())
}

fn has_database_option(config: &Config) -> bool {
    config.extra.iter().any(|arg| {
        arg == "--all-databases"
            || arg == "--databases"
            || arg.to_str().is_some_and(|s| s.starts_with("--databases="))
    })
}

fn database_from_options(config: &Config) -> Option<&str> {
    for (index, arg) in config.extra.iter().enumerate() {
        if arg == "--databases" {
            return config.extra.get(index + 1).and_then(|s| s.to_str());
        }
        if let Some(database) = arg.to_str().and_then(|s| s.strip_prefix("--databases=")) {
            return Some(database);
        }
    }
    None
}

fn validate(config: &Config) -> Result<(), Error> {
    let mysql = matches!(config.database, Database::MySql | Database::MariaDb);
    if config.db_name.is_empty() && !(mysql && has_database_option(config)) {
        return Err(invalid("db_name is required"));
    }
    if config.host.is_empty() && config.database != Database::Sqlite {
        return Err(invalid("host is required"));
    }
    if config.user_name.is_empty() && (mysql || config.database == Database::PostgreSql) {
        return Err(invalid("user_name is required"));
    }
    if mysql
        && [&config.user_name, &config.password, &config.host]
            .iter()
            .any(|s| s.contains('\0'))
    {
        return Err(invalid("MySQL credentials cannot contain NUL"));
    }
    if !config.include.is_empty() && !config.exclude.is_empty() {
        return Err(invalid("include_tables conflicts with exclude_tables"));
    }
    if config.append && config.compressor.is_some() {
        return Err(invalid("append mode conflicts with compression"));
    }
    if config.append && !mysql {
        return Err(invalid("append mode requires MySQL or MariaDB"));
    }
    if config.database == Database::MySql
        && !config.exclude_data.is_empty()
        && config.include_data
        && config
            .extra
            .iter()
            .chain(&config.extra_after)
            .any(|arg| arg == "--xml" || arg == "-X")
    {
        return Err(invalid(
            "MySQL table-data exclusions require SQL output, not XML",
        ));
    }
    if !config.ssl_flag.is_empty()
        && !["skip-ssl", "ssl-mode=DISABLED", "ssl-mode=PREFERRED"]
            .contains(&config.ssl_flag.as_str())
    {
        return Err(invalid("unsupported SSL flag"));
    }
    if mysql && config.db_name.starts_with('-') && !has_database_option(config) {
        return Err(invalid("database name must not begin with '-'"));
    }
    if mysql && config.include.iter().any(|table| table.starts_with('-')) {
        return Err(invalid("table names must not begin with '-'"));
    }
    if config.extra.iter().any(|s| s == "--databases")
        && database_from_options(config).is_none_or(|s| s.is_empty() || s.starts_with('-'))
    {
        return Err(invalid("--databases requires a separate database argument"));
    }
    if config.database == Database::PostgreSql && config.db_name.contains(['\r', '\n']) {
        return Err(invalid("PostgreSQL credentials cannot contain line breaks"));
    }
    Ok(())
}

fn credential_file(contents: &[u8]) -> Result<NamedTempFile, Error> {
    let mut file = NamedTempFile::new()?;
    file.write_all(contents)?;
    file.flush()?;
    Ok(file)
}

fn mysql_credentials(config: &Config) -> String {
    let escape = |s: &str| {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    };
    let mut content = format!(
        "[client]\nuser=\"{}\"\npassword=\"{}\"\nport={}\n",
        escape(&config.user_name),
        escape(&config.password),
        config.port
    );
    if config.socket.as_os_str().is_empty() {
        content.push_str(&format!("host=\"{}\"\n", escape(&config.host)));
    }
    if config.skip_ssl {
        let flag = if !config.ssl_flag.is_empty() {
            &config.ssl_flag
        } else if config.database == Database::MariaDb {
            "skip-ssl"
        } else {
            "ssl-mode=DISABLED"
        };
        content.push_str(flag);
        content.push('\n');
    }
    content
}

fn pg_credentials(config: &Config) -> Result<String, Error> {
    let host = if config.socket.as_os_str().is_empty() {
        config.host.as_str()
    } else {
        config
            .socket
            .to_str()
            .ok_or_else(|| invalid("PostgreSQL socket must be UTF-8"))?
    };
    let port = config.port.to_string();
    let entries = [
        host,
        &port,
        &config.db_name,
        &config.user_name,
        &config.password,
    ];
    if entries.iter().any(|s| s.contains(['\r', '\n', '\0'])) {
        return Err(invalid(
            "PostgreSQL credentials cannot contain line breaks or NUL",
        ));
    }
    Ok(entries
        .map(|s| s.replace('\\', "\\\\").replace(':', "\\:"))
        .join(":")
        + "\n")
}

fn run(
    command: &mut Command,
    input: File,
    output: File,
    config: &Config,
    start: Instant,
) -> Result<(), Error> {
    let remaining = if config.timeout.is_zero() {
        None
    } else {
        Some(
            config
                .timeout
                .checked_sub(start.elapsed())
                .filter(|d| !d.is_zero())
                .ok_or(Error::Timeout)?,
        )
    };
    let mut child = command
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output))
        .stderr(Stdio::null())
        .spawn()?;
    let status = if let Some(remaining) = remaining {
        match child.wait_timeout(remaining) {
            Ok(Some(status)) => status,
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::Timeout);
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        }
    } else {
        child.wait()?
    };
    if !status.success() {
        return Err(Error::ProcessFailed {
            program: command.get_program().to_string_lossy().into_owned(),
            code: status.code(),
        });
    }
    Ok(())
}

fn filter(input: &mut File, output: &mut File, config: &Config) -> Result<(), Error> {
    let mut reader = BufReader::new(input);
    let mut line = vec![];
    let mut first = true;
    while reader.read_until(b'\n', &mut line)? != 0 {
        if first
            && config.database == Database::MariaDb
            && !config.sandbox
            && line.starts_with(b"/*M!")
            && line.windows(14).any(|s| s == b"sandbox mode *")
        {
            first = false;
            line.clear();
            continue;
        }
        first = false;
        if config.skip_auto_increment && line.starts_with(b") ") {
            let needle = b" AUTO_INCREMENT=";
            if let Some(index) = line.windows(needle.len()).position(|s| s == needle) {
                let start = index + needle.len();
                let end = start
                    + line[start..]
                        .iter()
                        .take_while(|b| b.is_ascii_digit())
                        .count();
                if end > start
                    && line
                        .get(end)
                        .is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_')
                {
                    line.drain(index..end);
                }
            }
        }
        output.write_all(&line)?;
        line.clear();
    }
    Ok(())
}

pub(crate) fn sqlite_tables(config: &Config) -> Result<Vec<String>, Error> {
    let path = std::fs::canonicalize(&config.db_name)?;
    let mut command = Command::new(config.binary_path.join("sqlite3"));
    command.args([
        OsStr::new("-readonly"),
        OsStr::new("--bail"),
        OsStr::new("-json"),
        path.as_os_str(),
        OsStr::new(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT GLOB 'sqlite_*';",
        ),
    ]);
    let mut output = tempfile::tempfile()?;
    run(
        &mut command,
        tempfile::tempfile()?,
        output.try_clone()?,
        config,
        Instant::now(),
    )?;
    output.seek(SeekFrom::Start(0))?;
    let mut bytes = vec![];
    output.read_to_end(&mut bytes)?;
    if bytes.is_empty() {
        return Ok(vec![]);
    }
    let rows: Vec<std::collections::BTreeMap<String, String>> =
        serde_json::from_slice(&bytes).map_err(|_| invalid("invalid SQLite table listing"))?;
    Ok(rows
        .into_iter()
        .filter_map(|mut row| row.remove("name"))
        .collect())
}
