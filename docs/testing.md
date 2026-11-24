---
title: Testing
weight: 6
---

```bash
cargo test --locked
```

## Rust checks

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo test --locked --doc --all-features
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps --all-features
cargo package --locked
cargo +1.89.0 test --locked --all-targets --all-features
cargo +1.89.0 test --locked --doc --all-features
cargo audit --deny warnings
```

Install `cargo-audit` with `cargo install cargo-audit --locked` first. The portable tests compile a small Rust fixture executable. They need no running database and run on Linux, macOS and Windows in CI. The documentation's database examples compile as doctests with `no_run`; they retain the PHP examples' assumption that you supply database credentials and have the matching tool installed.

## Native database tests

Run these only against disposable databases named `db_dumper_test*`. They create and drop fixture tables; the MySQL/MariaDB tests also create and drop the named database. MongoDB's test replaces its `things` collection. The PostgreSQL test database must exist first.

SQLite tests need `sqlite3`, `gzip` and `bzip2`:

```bash
cargo test --locked --test native_tools sqlite_ -- --ignored --nocapture
```

The server tests use `DB_DUMPER_MYSQL_URL`, `DB_DUMPER_MARIADB_URL`, `DB_DUMPER_POSTGRESQL_URL` and `DB_DUMPER_MONGODB_URL`. Set these in your environment, not in source control. Install each database's client and dump executable. MongoDB's test also needs `mongorestore`. To point the MySQL test at a different client directory, put that directory on PATH and set `DB_DUMPER_MYSQL_URL_BINARY_PATH` to it.

```bash
cargo test --locked --test native_tools mysql_native_roundtrip -- --ignored --nocapture
cargo test --locked --test native_tools mariadb_native_roundtrip -- --ignored --nocapture
cargo test --locked --test native_tools postgresql_native_roundtrip -- --ignored --nocapture
cargo test --locked --test native_tools mongodb_native_archive_roundtrip -- --ignored --nocapture
```

The CI workflow provisions all four database services and runs a separate native-client job for each database. Native tests restore dumps and compare row values, not just file sizes. SQLite tests also cover selection, source/output aliases, gzip/bzip2 and a WAL snapshot while another connection writes. MySQL/MariaDB tests verify table-data exclusions, AUTO_INCREMENT filtering, and triggers that must not fire during restoration but must work afterwards.
