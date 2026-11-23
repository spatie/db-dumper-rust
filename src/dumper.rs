use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use percent_encoding::percent_decode_str;

use crate::{Compressor, DumpCommand, Error};

/// Supported native database dump tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Database {
    /// `mysqldump`.
    MySql,
    /// `mariadb-dump`.
    MariaDb,
    /// `pg_dump`.
    PostgreSql,
    /// `sqlite3`.
    Sqlite,
    /// `mongodump`.
    MongoDb,
}

/// Database selection for a typed or runtime-selected dumper.
pub trait DatabaseDriver: Clone {
    /// The native database family.
    fn database(&self) -> Database;
}

impl DatabaseDriver for Database {
    fn database(&self) -> Database {
        *self
    }
}

macro_rules! drivers {
    ($($name:ident => $driver:ident),* $(,)?) => {$ (
        #[doc = concat!("Driver selection for [`", stringify!($name), "`].")]
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $driver;
        impl DatabaseDriver for $driver {
            fn database(&self) -> Database { Database::$name }
        }
        #[doc = concat!("A ", stringify!($name), " database dumper. Start with `create()`.")]
        pub type $name = DbDumper<$driver>;
    )*};
}

drivers! {
    MySql => MySqlDriver,
    MariaDb => MariaDbDriver,
    PostgreSql => PostgreSqlDriver,
    Sqlite => SqliteDriver,
    MongoDb => MongoDbDriver,
}

/// Table names supplied as a collection or a comma-separated string.
#[derive(Debug, Clone)]
pub struct Tables(Vec<String>);

impl From<&str> for Tables {
    fn from(value: &str) -> Self {
        Self(
            value
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        )
    }
}

impl From<String> for Tables {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl<T: Into<String>, const N: usize> From<[T; N]> for Tables {
    fn from(value: [T; N]) -> Self {
        Self(value.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<String>> From<Vec<T>> for Tables {
    fn from(value: Vec<T>) -> Self {
        Self(value.into_iter().map(Into::into).collect())
    }
}

/// A consuming builder for native database dumps.
///
/// Configuration conflicts are reported by [`Self::get_dump_command`] or
/// [`Self::dump_to_file`]. Clones share an immutable compressor.
/// This type does not implement `Debug`, to avoid logging credentials.
#[derive(Clone)]
pub struct DbDumper<D = Database> {
    driver: D,
    pub(crate) config: Config,
}

#[derive(Clone)]
pub(crate) struct Config {
    pub database: Database,
    pub database_url: String,
    pub db_name: String,
    pub user_name: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub socket: PathBuf,
    pub timeout: Duration,
    pub binary_path: PathBuf,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub exclude_data: Vec<String>,
    pub extra: Vec<OsString>,
    pub extra_after: Vec<OsString>,
    pub compressor: Option<Arc<dyn Compressor>>,
    pub append: bool,
    pub create_tables: bool,
    pub include_data: bool,
    pub skip_ssl: bool,
    pub ssl_flag: String,
    pub skip_comments: bool,
    pub extended_inserts: bool,
    pub single_transaction: bool,
    pub skip_lock_tables: bool,
    pub no_column_statistics: bool,
    pub quick: bool,
    pub charset: String,
    pub gtid_purged: String,
    pub skip_auto_increment: bool,
    pub routines: bool,
    pub sandbox: bool,
    pub inserts: bool,
    pub collection: Option<String>,
    pub authentication_database: Option<String>,
}

impl<D: DatabaseDriver + Default> Default for DbDumper<D> {
    fn default() -> Self {
        Self::create()
    }
}

impl<D: DatabaseDriver + Default> DbDumper<D> {
    /// Create a dumper with the PHP package's defaults.
    pub fn create() -> Self {
        Self::with_driver(D::default())
    }
}

impl DbDumper<Database> {
    /// Select a database at runtime.
    pub fn new(database: Database) -> Self {
        Self::with_driver(database)
    }
}

macro_rules! string_setters {
    ($($name:ident => $field:ident: $doc:literal),* $(,)?) => {$ (
        #[doc = $doc]
        pub fn $name(mut self, value: impl Into<String>) -> Self {
            self.config.$field = value.into(); self
        }
    )*};
}

macro_rules! switches {
    ($($name:ident => $field:ident = $value:tt: $doc:literal),* $(,)?) => {$ (
        #[doc = $doc]
        pub fn $name(mut self) -> Self { self.config.$field = $value; self }
    )*};
}

impl<D: DatabaseDriver> DbDumper<D> {
    fn with_driver(driver: D) -> Self {
        let database = driver.database();
        Self {
            driver,
            config: Config {
                database,
                database_url: String::new(),
                db_name: String::new(),
                user_name: String::new(),
                password: String::new(),
                host: "localhost".into(),
                port: match database {
                    Database::MySql | Database::MariaDb => 3306,
                    Database::MongoDb => 27017,
                    _ => 5432,
                },
                socket: PathBuf::new(),
                timeout: Duration::ZERO,
                binary_path: PathBuf::new(),
                include: vec![],
                exclude: vec![],
                exclude_data: vec![],
                extra: vec![],
                extra_after: vec![],
                compressor: None,
                append: false,
                create_tables: true,
                include_data: true,
                skip_ssl: false,
                ssl_flag: String::new(),
                skip_comments: true,
                extended_inserts: true,
                single_transaction: false,
                skip_lock_tables: false,
                no_column_statistics: false,
                quick: false,
                charset: String::new(),
                gtid_purged: "AUTO".into(),
                skip_auto_increment: false,
                routines: false,
                sandbox: true,
                inserts: false,
                collection: None,
                authentication_database: None,
            },
        }
    }

    string_setters! {
        set_db_name => db_name: "Set the database name, or SQLite file path.",
        set_user_name => user_name: "Set the database user name.",
        set_password => password: "Set the password. Empty passwords are allowed.",
        set_host => host: "Set the host. Defaults to localhost.",
        set_default_character_set => charset: "Set MySQL/MariaDB's default character set.",
        set_gtid_purged => gtid_purged: "Set MySQL's GTID_PURGED mode. AUTO emits no flag.",
        set_ssl_flag => ssl_flag: "Override MySQL/MariaDB's SSL flag; checked when preparing a dump.",
    }

    switches! {
        do_not_create_tables => create_tables = false: "Dump data without CREATE TABLE statements (MySQL/MariaDB/PostgreSQL).",
        do_not_dump_data => include_data = false: "Dump schema without row data (MySQL/MariaDB/PostgreSQL).",
        use_append_mode => append = true: "Append a successful MySQL/MariaDB dump to the destination.",
        skip_comments => skip_comments = true: "Omit MySQL/MariaDB comments (the default).",
        do_not_skip_comments => skip_comments = false: "Keep MySQL/MariaDB comments.",
        use_extended_inserts => extended_inserts = true: "Use multi-row MySQL/MariaDB INSERT statements (the default).",
        do_not_use_extended_inserts => extended_inserts = false: "Use one MySQL/MariaDB INSERT per row.",
        use_single_transaction => single_transaction = true: "Use a single MySQL/MariaDB transaction.",
        do_not_use_single_transaction => single_transaction = false: "Do not request a single transaction.",
        skip_lock_tables => skip_lock_tables = true: "Do not lock MySQL/MariaDB tables.",
        do_not_skip_lock_tables => skip_lock_tables = false: "Do not disable MySQL/MariaDB table locks.",
        do_not_use_column_statistics => no_column_statistics = true: "Disable MySQL column statistics.",
        use_quick => quick = true: "Retrieve MySQL/MariaDB rows incrementally.",
        do_not_use_quick => quick = false: "Do not explicitly request quick retrieval.",
        skip_auto_increment => skip_auto_increment = true: "Remove MySQL/MariaDB AUTO_INCREMENT table counters.",
        do_not_skip_auto_increment => skip_auto_increment = false: "Keep AUTO_INCREMENT counters (the default).",
        include_routines => routines = true: "Include MySQL/MariaDB stored procedures and functions.",
        without_sandbox_mode => sandbox = false: "Remove MariaDB's leading sandbox directive if present.",
        use_inserts => inserts = true: "Use PostgreSQL INSERT statements instead of COPY.",
    }

    /// The selected database family.
    pub fn database(&self) -> Database {
        self.driver.database()
    }
    /// The configured database name or SQLite path.
    pub fn get_db_name(&self) -> &str {
        &self.config.db_name
    }
    /// The configured database URL. May contain credentials; do not log it.
    pub fn get_database_url(&self) -> &str {
        &self.config.database_url
    }
    /// The configured host.
    pub fn get_host(&self) -> &str {
        &self.config.host
    }
    /// The compressor extension, or an empty string when uncompressed.
    pub fn get_compressor_extension(&self) -> &str {
        self.config
            .compressor
            .as_ref()
            .map_or("", |c| c.use_extension())
    }
    /// Set the TCP port.
    pub fn set_port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }
    /// Disable MySQL/MariaDB SSL, or restore the native client's default.
    pub fn set_skip_ssl(mut self, skip: bool) -> Self {
        self.config.skip_ssl = skip;
        self
    }
    /// Set a MySQL socket or PostgreSQL socket directory.
    pub fn set_socket(mut self, socket: impl Into<PathBuf>) -> Self {
        self.config.socket = socket.into();
        self
    }
    /// Set the total native-process deadline. Zero means no deadline.
    pub fn set_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }
    /// Set the directory containing the native dump executable.
    pub fn set_dump_binary_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.binary_path = path.into();
        self
    }
    /// Compress the completed dump using an external stdin/stdout compressor.
    pub fn use_compressor(mut self, compressor: impl Compressor + 'static) -> Self {
        self.config.compressor = Some(Arc::new(compressor));
        self
    }
    /// Include only these tables. Cannot be combined with exclusions.
    pub fn include_tables(mut self, tables: impl Into<Tables>) -> Self {
        self.config.include = tables.into().0;
        self
    }
    /// Exclude these tables. Cannot be combined with inclusions.
    pub fn exclude_tables(mut self, tables: impl Into<Tables>) -> Self {
        self.config.exclude = tables.into().0;
        self
    }
    /// Exclude the data, but not the schema, of these MySQL/MariaDB tables.
    pub fn exclude_tables_data(mut self, tables: impl Into<Tables>) -> Self {
        self.config.exclude_data = tables.into().0;
        self
    }
    /// Add one literal argument before the database name. No shell splitting occurs.
    pub fn add_extra_option(mut self, option: impl Into<OsString>) -> Self {
        let option = option.into();
        if !option.is_empty() {
            self.config.extra.push(option);
        }
        self
    }
    /// Add one literal argument after the database name and table selection.
    pub fn add_extra_option_after_db_name(mut self, option: impl Into<OsString>) -> Self {
        let option = option.into();
        if !option.is_empty() {
            self.config.extra_after.push(option);
        }
        self
    }
    /// Dump one MongoDB collection.
    pub fn set_collection(mut self, collection: impl Into<String>) -> Self {
        self.config.collection = Some(collection.into());
        self
    }
    /// Set MongoDB's authentication database.
    pub fn set_authentication_database(mut self, database: impl Into<String>) -> Self {
        self.config.authentication_database = Some(database.into());
        self
    }
    /// Parse a database URL. Query components override URL components.
    /// Missing components leave existing settings unchanged.
    pub fn set_database_url(mut self, value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        let normalized = value.replacen("sqlite:///", "sqlite://null/", 1).replacen(
            "sqlite3:///",
            "sqlite3://null/",
            1,
        );
        let url = url::Url::parse(&normalized).map_err(|_| Error::InvalidDatabaseUrl)?;
        let decode = |value: &str| {
            percent_decode_str(value)
                .decode_utf8()
                .map(|s| s.into_owned())
                .map_err(|_| Error::InvalidDatabaseUrl)
        };
        let mut components = std::collections::BTreeMap::new();
        if let Some(host) = url.host_str().filter(|s| *s != "null") {
            components.insert("host".to_owned(), decode(host)?);
        }
        if let Some(port) = url.port() {
            components.insert("port".into(), port.to_string());
        }
        if !url.username().is_empty() {
            components.insert("username".into(), decode(url.username())?);
        }
        if let Some(password) = url.password() {
            components.insert("password".into(), decode(password)?);
        }
        let database = if url.scheme().contains("sqlite") {
            url.path()
        } else {
            url.path().trim_matches('/')
        };
        if !database.is_empty() {
            components.insert("database".into(), decode(database)?);
        }
        components.extend(
            url.query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned())),
        );
        for (key, value) in components {
            if value.is_empty() || value == "null" {
                continue;
            }
            match key.as_str() {
                "host" => self.config.host = value,
                "port" => {
                    self.config.port = value.parse().map_err(|_| Error::InvalidDatabaseUrl)?
                }
                "database" => self.config.db_name = value,
                "username" => self.config.user_name = value,
                "password" => self.config.password = value,
                _ => {}
            }
        }
        self.config.database_url = value;
        Ok(self)
    }
    /// Prepare a shell-free command, owning any temporary credential files.
    pub fn get_dump_command(&self) -> Result<DumpCommand, Error> {
        DumpCommand::prepare(self.config.clone())
    }
    /// Run the dump and save it only after all native processes succeed.
    pub fn dump_to_file(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        self.get_dump_command()?.dump_to_file(path)
    }
    /// List user tables in a SQLite database using the configured SQLite executable.
    pub fn get_db_tables(&self) -> Result<Vec<String>, Error> {
        if self.database() != Database::Sqlite {
            return Err(Error::InvalidConfiguration(
                "get_db_tables requires SQLite".into(),
            ));
        }
        crate::process::sqlite_tables(&self.config)
    }
}
