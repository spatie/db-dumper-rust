---
title: Requirements
weight: 3
---

For dumping MySQL-db's `mysqldump` should be installed.

For dumping MariaDB-db's `mariadb-dump` should be installed.

For dumping PostgreSQL-db's `pg_dump` should be installed.

For dumping SQLite-db's `sqlite3` should be installed.

For dumping MongoDB-db's `mongodump` should be installed.

For compressing dump files, `gzip` and/or `bzip2` should be installed.

The crate requires Rust 1.89 or newer. Use SQLite 3.33+ for JSON table listings and MongoDB Database Tools 100.3.0+ for protected password configuration. Choose a dump client compatible with your database server. No PHP, `sed` or `tail` installation is needed.
