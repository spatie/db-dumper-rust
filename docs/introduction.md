---
title: Introduction
weight: 1
---

This repo contains an easy to use crate to dump a database using Rust. Currently MySQL, MariaDB, PostgreSQL, SQLite and MongoDB are supported. Behind the scenes `mysqldump`, `mariadb-dump`, `pg_dump`, `sqlite3` and `mongodump` are used.

Here are simple examples of how to create a database dump with different drivers:

**MySQL**

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

**MariaDB**

```rust,no_run
use spatie_db_dumper::{MariaDb};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MariaDb::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

**PostgreSQL**

```rust,no_run
use spatie_db_dumper::{PostgreSql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
PostgreSql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

**SQLite**

```rust,no_run
use spatie_db_dumper::{Sqlite};

# let path_to_database_file = "database.sqlite";
Sqlite::create()
    .set_db_name(path_to_database_file)
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

**MongoDB**

```rust,no_run
use spatie_db_dumper::{MongoDb};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MongoDb::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .dump_to_file("dump.gz")?;
# Ok::<(), spatie_db_dumper::Error>(())
```
