# PHP parity

This port follows the standalone [spatie/db-dumper](https://github.com/spatie/db-dumper) package at commit `bf35e0b286ae458735acfec8e183be18ffaa98d1`. The PHP repository has no Markdown guide tree; its README supplies the documentation's sections, headings, order, prose and matching examples. The guide pages split those sections without changing their order. The root guide is labelled v1; the crate version is 0.1.0.

## Database drivers

| PHP | Rust | Native executable | Default host / port |
| --- | --- | --- | --- |
| `MySql::create()` | `MySql::create()` | `mysqldump` | localhost / 3306 |
| `MariaDb::create()` | `MariaDb::create()` | `mariadb-dump` | localhost / 3306 |
| `PostgreSql::create()` | `PostgreSql::create()` | `pg_dump` | localhost / 5432 |
| `Sqlite::create()` | `Sqlite::create()` | `sqlite3` | local file |
| `MongoDb::create()` | `MongoDb::create()` | `mongodump` | localhost / 27017 |

`DbDumper::new(Database::MySql)` selects a driver at runtime. The named types are typed aliases of the same generic builder. Every builder implements `Clone`; a cloned builder shares its immutable compressor through `Arc`. Setters consume and return `Self`, except `set_database_url`, which returns `Result<Self, Error>`. Hooks receiving `&mut DbDumper` can assign `*dumper = dumper.clone().use_single_transaction()`.

## Complete option mapping

Unless noted, PHP camelCase names map directly to the following snake_case names. Settings retain PHP defaults. Methods apply to the same database families as PHP; an inherited setting PHP ignores is also ignored here (for example MongoDB's inherited table selectors and extra options, or SQLite's inherited schema/data flags). Use MongoDB's `set_collection` for collection selection. The common builder deliberately exposes the same interface for runtime-selected drivers.

| PHP option or operation | Rust API | Scope / default |
| --- | --- | --- |
| `setDbName`, `getDbName` | `set_db_name(impl Into<String>)`, `get_db_name() -> &str` | All; empty |
| `setDatabaseUrl`, `getDatabaseUrl` | `set_database_url(impl Into<String>) -> Result<Self, Error>`, `get_database_url() -> &str` | All; query components override URL components |
| `setUserName` | `set_user_name(impl Into<String>)` | Server databases; empty |
| `setPassword` | `set_password(impl Into<String>)` | Server databases; empty |
| `setHost`, `getHost` | `set_host(impl Into<String>)`, `get_host() -> &str` | Server databases; localhost |
| `setPort` | `set_port(u16)` | Server databases; defaults above |
| `setSocket` | `set_socket(impl Into<PathBuf>)` | MySQL/MariaDB socket, PostgreSQL socket directory |
| `setTimeout` | `set_timeout(Duration)` | Zero disables deadline |
| `setDumpBinaryPath` | `set_dump_binary_path(impl Into<PathBuf>)` | Directory; empty searches PATH |
| `includeTables` | `include_tables(impl Into<Tables>)` | MySQL/MariaDB/PostgreSQL/SQLite |
| `excludeTables` | `exclude_tables(impl Into<Tables>)` | MySQL/MariaDB/PostgreSQL/SQLite |
| `excludeTablesData` | `exclude_tables_data(impl Into<Tables>)` | MySQL/MariaDB |
| `addExtraOption` | `add_extra_option(impl Into<OsString>)` | MySQL/MariaDB/PostgreSQL; one literal argument |
| `addExtraOptionAfterDbName` | `add_extra_option_after_db_name(impl Into<OsString>)` | MySQL/MariaDB; one literal argument |
| `doNotCreateTables` | `do_not_create_tables()` | MySQL/MariaDB/PostgreSQL |
| `doNotDumpData` | `do_not_dump_data()` | MySQL/MariaDB/PostgreSQL |
| `useAppendMode` | `use_append_mode()` | MySQL/MariaDB; false |
| `useCompressor` | `use_compressor(impl Compressor + 'static)` | All; none |
| `getCompressorExtension` | `get_compressor_extension() -> &str` | Empty, gz, bz2, or custom extension |
| `setSkipSsl` | `set_skip_ssl(bool)` | MySQL/MariaDB; false |
| `setSslFlag` | `set_ssl_flag(impl Into<String>)` | skip-ssl, ssl-mode=DISABLED, ssl-mode=PREFERRED; takes effect only when skip_ssl is true |
| `skipComments`, `doNotSkipComments` | `skip_comments()`, `do_not_skip_comments()` | MySQL/MariaDB; skip by default |
| `useExtendedInserts`, `doNotUseExtendedInserts` | `use_extended_inserts()`, `do_not_use_extended_inserts()` | MySQL/MariaDB; true |
| `useSingleTransaction`, `doNotUseSingleTransaction` | `use_single_transaction()`, `do_not_use_single_transaction()` | MySQL/MariaDB; false |
| `skipLockTables`, `doNotSkipLockTables` | `skip_lock_tables()`, `do_not_skip_lock_tables()` | MySQL/MariaDB; false |
| `doNotUseColumnStatistics` | `do_not_use_column_statistics()` | MySQL/MariaDB; false |
| `useQuick`, `doNotUseQuick` | `use_quick()`, `do_not_use_quick()` | MySQL/MariaDB; false |
| `setDefaultCharacterSet` | `set_default_character_set(impl Into<String>)` | MySQL/MariaDB; unset |
| `setGtidPurged` | `set_gtid_purged(impl Into<String>)` | MySQL/MariaDB; AUTO emits no flag |
| `skipAutoIncrement`, `doNotSkipAutoIncrement` | `skip_auto_increment()`, `do_not_skip_auto_increment()` | MySQL/MariaDB; false |
| `includeRoutines` | `include_routines()` | MySQL/MariaDB; false |
| `withoutSandboxMode` | `without_sandbox_mode()` | MariaDB; sandbox enabled by default |
| `useInserts` | `use_inserts()` | PostgreSQL; false |
| `setCollection` | `set_collection(impl Into<String>)` | MongoDB; unset |
| `setAuthenticationDatabase` | `set_authentication_database(impl Into<String>)` | MongoDB; unset |
| `dumpToFile` | `dump_to_file(impl AsRef<Path>) -> Result<(), Error>` | All |
| `getDbTables` | `get_db_tables() -> Result<Vec<String>, Error>` | SQLite; excludes sqlite_% internal tables |
| No PHP equivalent | `database() -> Database` | Selected driver |

`Tables` accepts `&str`, `String`, `[T; N]` and `Vec<T>` where `T: Into<String>`. String input splits on commas and trims whitespace. Array entries are literal names/patterns. Empty selectors mean no filtering, as in PHP. Include and exclude selectors cannot both be nonempty. Excluding table data can be combined with either selector.

`GzipCompressor` and `Bzip2Compressor` implement `Compressor: Send + Sync`. Its required methods are `program() -> PathBuf` and `use_extension() -> &str`; `args() -> Vec<OsString>` defaults to no arguments. The external program must read stdin, write the compressed dump to stdout and return a nonzero status on failure. File extensions are informational: `dump_to_file` does not add or infer one. In particular the upstream MongoDB example's `dump.gz` is an uncompressed archive unless a compressor is selected.

## Command preparation and errors

`get_dump_command() -> Result<DumpCommand, Error>` replaces PHP's shell-string `getDumpCommand` and Symfony `getProcess`. It validates configuration and owns temporary credentials. Inspect its `program()`, `args()`, `additional_args()`, `environment()` and `stdin()`; call its `dump_to_file(path)` to execute it. `additional_args()` contains subsequent MySQL data/trigger passes, or is empty for ordinary dumps. The output path and a credentials-file path are not arguments to preparation. The caller no longer creates a credentials file or controls a temporary handle. Dropping a prepared command removes its credentials. Keep the prepared command alive while using its paths.

PHP's `getContentsOfCredentialsFile`, `getTempFileHandle`, `setTempFileHandle`, `guardAgainstIncompleteCredentials` and `checkIfDumpWasSuccessful` support its process implementation. They are not exposed as separate Rust operations: preparation and execution perform them automatically. `DsnParser` is internal to `set_database_url`, not a public general-purpose URL parser.

`Error` has these public variants:

- `InvalidConfiguration(String)` for missing user/database/host, conflicting include/exclude, append/compressor conflicts, invalid SSL flags and rejected names or credentials.
- `InvalidDatabaseUrl` without the URL, because it may contain a password.
- `Io(std::io::Error)` for missing executables and failed file operations.
- `ProcessFailed { program: String, code: Option<i32> }` for a failed dump tool or compressor; signal termination has no exit code.
- `Timeout` after killing and reaping the direct child.
- `EmptyDump` for a successful process that produced no data, including empty filtered/compressed results.

## Deliberate differences

1. No shell evaluation or splitting occurs. Paths, credentials, host names, patterns and extra options are literal. Replace `addExtraOption('--databases dbname')` with `.add_extra_option("--databases").add_extra_option("dbname")`. Shell pipes and redirects are not supported extra options. A custom compressor is an executable plus arguments, not a shell fragment. Extra options still grant the native program's full capabilities; only accept them from trusted configuration.
2. PHP exceptions become `Result`. Most setters cannot fail; conflicts and missing settings are checked at preparation/execution, not at the first conflicting setter. A later setter may therefore resolve a conflict. Timeouts use `Duration`; ports use `u16`; paths use Rust path types. Boolean SSL selection requires an explicit bool. The binary-path setter takes an explicit argument, including `""` to reset it. Builders are consuming rather than mutable PHP objects.
3. MySQL credential values escape quotes, backslashes, line breaks and tabs in a protected option file; NUL is rejected. The default SSL-disable flag is `ssl-mode=DISABLED` for MySQL and `skip-ssl` for MariaDB. Invalid SSL overrides return an error instead of being silently ignored. Database names and selected MySQL/MariaDB tables beginning with `-` are rejected to prevent their interpretation as flags. `--all-databases` is matched as a whole argument, not a substring inside another option. `--databases` uses its separate following argument, or `--databases=dbname`, and does not mutate `get_db_name()`. Exclusions use that effective database name.
4. PostgreSQL uses a protected escaped pgpass file and `PGDATABASE`, as PHP does. `-w` disables interactive password prompts. CR/LF/NUL credentials are rejected rather than allowing truncated or extra pgpass entries. The socket directory is also used in the pgpass entry. Inherited `PGPASSWORD`, `PGSERVICE` and `PGSERVICEFILE` are removed so they cannot silently override selected credentials. Other environment settings remain inherited.
5. MongoDB passwords use a protected YAML config file rather than command-line arguments. This requires MongoDB Database Tools 100.3.0 or newer. User names, host, port, collection and authentication database retain their PHP semantics. Built-in credentials do not appear in command arguments. Extra arguments may contain secrets and must not be logged.
6. There is no PHP-style JSON scalar coercion in database URLs: names/passwords such as `0` and `false` remain literal strings rather than becoming falsey values and disappearing. Empty and literal `null` values retain PHP's leave-existing-setting behavior. Invalid/non-numeric/out-of-range ports return an error instead of being coerced to zero. Query values are strings, not PHP arrays. URL parsing uses Rust's `url` crate, including its URL normalization and percent-decoding; it is not PHP's permissive `parse_url` implementation.
7. Dumps are staged in private temporary files beside the destination and published only after the dump, filters and compressor succeed. An existing output survives process, timeout and compression failures. All temporary files are removed by RAII. This changes PHP's direct-write behavior and requires free space for the uncompressed dump and, when filtering/compressing, another file of the corresponding size. Compression is a separate process reading the completed dump, not a concurrent shell pipeline. Both exit statuses and nonempty raw output are checked, so an empty dump cannot masquerade as a valid compressed header.
8. Append remains available for MySQL/MariaDB, but only a successful complete dump is appended. As in PHP it cannot be combined with compression. Append publication is not atomic: a disk/write failure can leave a partial append, and concurrent append writers are not synchronized. Overwrite publication replaces the destination file, rather than truncating its inode; permissions/ownership/ACLs from an old output are not preserved. New files use tempfile's private permissions (0600 on Unix; the caller's temp/destination directory ACLs apply on Windows). Existing output symlinks are rejected. No fsync/durability guarantee is made.
9. SQLite opens an existing database read-only and starts `BEGIN;` before `.dump`, preserving PHP's WAL snapshot behavior. A missing source returns an error instead of creating an empty SQLite database. Preparation resolves the source path, so the file must exist even when only inspecting the command. Source/output aliases, including hard links, are rejected. SQLite's `-json` table listing replaces the PHP SQLite extension; SQLite 3.33+ is required. Excluding every table emits an empty transaction instead of accidentally dumping every table. SQLite dot-command patterns are quoted; CR/LF/NUL patterns are rejected to prevent injection of another SQLite command. Native wildcard semantics remain in effect. An exclusion table listing is a separate read before the dump, as in PHP; concurrent schema changes can change that selection.
10. AUTO_INCREMENT removal runs in Rust on the closing table-option lines of ordinary mysqldump SQL, not `sed` over every line. It preserves matching text inside INSERT data. MariaDB sandbox removal only removes a leading sandbox directive when present; PHP's `tail +2` would also delete an unrelated first line. Neither filter requires `sed` or `tail`, and the underlying dump status is not hidden by a successful filter.
11. Process stderr is discarded, not included in errors, to avoid credential leaks from native tool diagnostics. `DbDumper` and `DumpCommand` do not implement `Debug`. Errors retain the executable and exit status, not the command string or its output. Passwords are ordinary owned Rust strings, not zeroized memory. Temporary-file permissions do not protect against an administrator or a process with the same account privileges.
12. Zero disables timeouts. Dump and compression processes share an elapsed-time budget; SQLite exclusion-table preparation has its own timed query. Timed-out direct children are killed and reaped, but independently spawned descendants are not. The timeout does not interrupt local file filtering/copying. There is no async runtime, cancellation callback, PHP inheritance, or Symfony process object. The package remains a standalone library, not a CLI.
13. PHP emits MariaDB's `--ignore-table-data` even for Oracle MySQL, where it is not supported (verified with Oracle mysqldump 8.4.13 and its [option reference](https://dev.mysql.com/doc/refman/8.4/en/mysqldump.html)). Rust keeps MariaDB's native flag, but MySQL table-data exclusions use schema, data and trigger passes, in that order. Creating triggers last prevents them firing while restoring the data. Only the data pass dumps rows, so `use_single_transaction` still gives those rows one consistent snapshot. Schema and trigger metadata come from separate connections: do not change DDL during this operation. Later passes do not duplicate routines/events or GTID_PURGED. A failure in any pass discards the whole staged dump. Data-only dumps need just one pass with fully excluded tables; schema-only dumps need no special exclusion. This fallback requires SQL output; combining it with `--xml` or `-X` returns an error rather than concatenating invalid XML documents. A bare exclusion name applies to the configured/effective database, as in PHP, not to every database selected by `--all-databases`.

Use trusted source and destination directories. Path checks and publication are not a sandbox against hostile concurrent file replacement. Native-tool flags and supported server versions remain the installed tool's responsibility, as in PHP; for example MariaDB-specific or recent MySQL options require a compatible dump binary.

## Verification

The portable suite executes Rust fixture tools to test exact arguments, process failures, timeouts, large output, compression failure, atomic replacement and literal shell-looking values on every CI OS. Native integration tests dump and restore real databases, compare restored rows and verify SQLite WAL snapshots and both compression formats. Run them only against disposable `db_dumper_test*` databases; they drop fixture tables and databases. See [testing](docs/testing.md) and the CI workflow for commands and required services. README and usage examples are compiled as doctests; database-dependent examples use `no_run`, since their PHP counterparts assume the caller's credentials and database. They are not claimed as live-database test executions.
