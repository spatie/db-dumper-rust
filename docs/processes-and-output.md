---
title: Processes and output safety
weight: 12
---

## Commands and credentials

`get_dump_command()` validates the configuration and returns a `DumpCommand` owning its temporary credentials. Inspect `program()`, `args()`, `additional_args()`, `environment()` and `stdin()`, or execute it with `dump_to_file(path)`. Keep the prepared command alive while using its credential paths. Dropping it removes those files.

Arguments are literal. Pass `.add_extra_option("--databases").add_extra_option("dbname")`, not a shell command string. Pipes, redirects and shell expansion are not supported. Extra options still grant the native program's full capabilities, so accept them only from trusted configuration.

MySQL and MariaDB use protected option files. PostgreSQL uses an escaped pgpass file and `PGDATABASE`, with interactive password prompts disabled. Inherited `PGPASSWORD`, `PGSERVICE` and `PGSERVICEFILE` are removed. MongoDB passwords use a protected YAML configuration file, requiring Database Tools 100.3.0 or newer. Do not log extra arguments that contain secrets. Process stderr is discarded to avoid credential leaks; errors retain the executable and exit status. Password strings are not zeroized.

## Output publication

Dumps are staged in private temporary files beside the destination. An existing output survives process, timeout, filter and compression failures. Allow space for the uncompressed dump and another file when filtering or compressing. Compression reads the completed dump, not a concurrent pipeline. Compressors must read stdin, write stdout and return a nonzero exit status on failure. Output extensions are not added automatically.

Overwrite publication replaces the destination file. Existing ownership, permissions and ACLs are not preserved. New files use private temporary-file permissions (0600 on Unix; destination directory ACLs apply on Windows). Existing output symlinks are rejected. There is no `fsync` or power-loss durability guarantee.

MySQL and MariaDB append mode appends only a successful complete dump and cannot be combined with compression. Append is not atomic. Write failures can leave a partial append, and concurrent writers are not synchronized.

Use trusted source and destination directories. Path checks do not protect against hostile concurrent file replacement.

## Driver details

SQLite opens an existing source read-only and begins a transaction before `.dump`. The source must exist even for command inspection. Source/output aliases, including hard links, are rejected. Exclusion-table listing is a separate read, so concurrent schema changes can affect selection. Native wildcard semantics apply; CR, LF and NUL in patterns are rejected. Excluding every table produces an empty transaction.

MySQL table-data exclusions use schema, data and trigger passes in that order. The data pass uses one snapshot when `use_single_transaction` is enabled; metadata uses separate connections, so avoid concurrent DDL. Any pass failure discards the staged dump. This requires SQL output and rejects `--xml` and `-X`. MariaDB uses its native `--ignore-table-data` flag. A bare excluded name refers to the effective database, not every database selected by `--all-databases`.

AUTO_INCREMENT removal changes closing table-option lines, not matching INSERT data. MariaDB sandbox removal affects only a leading sandbox directive. Neither filter needs `sed` or `tail`.

## Errors and deadlines

Most setters are infallible. Missing settings and conflicts are checked when preparing or executing, so a later setter can resolve a conflict. `set_database_url` returns a `Result`; literal `0` and `false` credentials remain strings. Empty and literal `null` values leave an existing setting unchanged. Invalid ports return an error.

Errors distinguish invalid configuration, invalid database URLs, I/O failures, failed processes, timeouts and empty dumps. Database URLs are omitted from errors because they can contain passwords.

Zero disables timeouts. Dump and compression children share an elapsed-time budget; SQLite exclusion-table preparation has its own timed query. Timed-out direct children are killed and reaped. Independently spawned descendants and local filtering/copying are not interrupted.
