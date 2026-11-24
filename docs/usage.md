---
title: Usage
weight: 5
---

Builder methods consume and return the dumper. Dumping returns `Result<(), Error>`. Extra options are literal arguments, not shell fragments. See [processes and output safety](processes-and-output.md) for credentials, deadlines and publication guarantees.

This is the simplest way to create a dump of a MySql db:

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

If you're working with PostgreSQL just use that dumper, most methods are available on both the MySql. and PostgreSql-dumper.

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

If the `mysqldump` (or `pg_dump`) binary is installed in a non default location you can let the package know by using the`set_dump_binary_path()`-function:

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_dump_binary_path("/custom/location")
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

If your application is deployed and you need to change the host (default is 127.0.0.1), you can add the `set_host()`-function:

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
# let host = "hostname";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .set_host(host)
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Handling AUTO_INCREMENT Values in Dumps

When creating a database dump, you might need to control the inclusion of AUTO_INCREMENT values. This can be crucial for avoiding primary key conflicts or for maintaining ID consistency when transferring data across environments.

#### Skipping AUTO_INCREMENT Values

To omit the AUTO_INCREMENT values from the tables in your dump, use the skip_auto_increment method. This is particularly useful to prevent conflicts when importing the dump into another database where those specific AUTO_INCREMENT values might already exist, or when the exact values are not relevant.

```rust,no_run
use spatie_db_dumper::{MySql};

MySql::create()
    .set_db_name("dbname")
    .set_user_name("username")
    .set_password("password")
    .skip_auto_increment()
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Including AUTO_INCREMENT values in the dump

By default, the AUTO_INCREMENT values are included in the dump. However, if you previously used the `skip_auto_increment` method and wish to ensure that the AUTO_INCREMENT values are included in a subsequent dump, use the `do_not_skip_auto_increment` method to explicitly include them.

```rust,no_run
use spatie_db_dumper::{MySql};

MySql::create()
    .set_db_name("dbname")
    .set_user_name("username")
    .set_password("password")
    .do_not_skip_auto_increment()
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Use a Database URL

In some applications or environments, database credentials are provided as URLs instead of individual components. In this case, you can use the `set_database_url` method instead of the individual methods.

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_url = "mysql://username:password@hostname:3306/dbname";
MySql::create()
    .set_database_url(database_url)?
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

When providing a URL, the package will automatically parse it and provide the individual components to the applicable dumper.

For example, if you provide the URL `mysql://username:password@hostname:3306/dbname`, the dumper will use the `hostname` host, running on port `3306`, and will connect to `dbname` with `username` and `password`.

### Dump specific tables

Using an array:

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .include_tables(["table1", "table2", "table3"])
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

Using a string:

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .include_tables("table1, table2, table3")
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Don't use column_statics table with some old version of MySql service.

In order to use "_--column-statistics=0_" as option in mysqldump command you can use _do_not_use_column_statistics()_ method.

If you have installed _mysqldump 8_, it queries by default _column_statics_ table in _information_schema_ database.
In some old version of MySql (service) like 5.7, this table doesn't exist. So you could have an exception during the execution of mysqldump. To avoid this, you could use _do_not_use_column_statistics()_ method.

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .do_not_use_column_statistics()
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Excluding tables from the dump

You can exclude tables from the dump by using an array:

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .exclude_tables(["table1", "table2", "table3"])
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

Or by using a string:

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .exclude_tables("table1, table2, table3")
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Excluding table data from the dump

If you want to dump the structure of certain tables but not their data (e.g. log tables), you can use `exclude_tables_data`. This works with both MySQL and MariaDB.

Using an array:

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .exclude_tables_data(["logs", "sessions"])
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

You can combine this with `exclude_tables` to fully exclude some tables and only skip data for others:

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .exclude_tables(["cache"])
    .exclude_tables_data(["logs", "sessions"])
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Including routines in the dump

To include stored procedures and functions in your MySQL or MariaDB dump, use the `include_routines` method:

```rust,no_run
use spatie_db_dumper::{MySql};

# let database_name = "dbname";
# let user_name = "username";
# let password = "password";
MySql::create()
    .set_db_name(database_name)
    .set_user_name(user_name)
    .set_password(password)
    .include_routines()
    .dump_to_file("dump.sql")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Do not write CREATE TABLE statements that create each dumped table

You can use `do_not_create_tables` to prevent writing create statements.

```rust,no_run
use spatie_db_dumper::{MySql};

let dump_command = MySql::create()
    .set_db_name("dbname")
    .set_user_name("username")
    .set_password("password")
    .do_not_create_tables()
    .get_dump_command()?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Do not write row data

You can use `do_not_dump_data` to prevent writing row data.


```rust,no_run
use spatie_db_dumper::{MySql};

let dump_command = MySql::create()
    .set_db_name("dbname")
    .set_user_name("username")
    .set_password("password")
    .do_not_dump_data()
    .get_dump_command()?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Append instead of overwriting a dump file

You can use `use_append_mode` with MySQL to append to the file instead of overwriting it.
This is useful for two-step dumps when you want to dump the whole schema but only some of the data
(for example: only migrations, or only product but not customer data).

```rust,no_run
use spatie_db_dumper::{MySql};

let dump_command = MySql::create()
    .set_db_name("dbname")
    .set_user_name("username")
    .set_password("password")
    .use_append_mode()
    .get_dump_command()?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Adding extra options

If you want to add an arbitrary option to the dump command you can use `add_extra_option`

```rust,no_run
use spatie_db_dumper::{MySql};

let dump_command = MySql::create()
    .set_db_name("dbname")
    .set_user_name("username")
    .set_password("password")
    .add_extra_option("--xml")
    .get_dump_command()?;
# Ok::<(), spatie_db_dumper::Error>(())
```

If you're working with MySql you can set the database name using `--databases` as an extra option. This is particularly useful when used in conjunction with the `--add-drop-database` `mysqldump` option (see the [mysqldump docs](https://dev.mysql.com/doc/refman/5.7/en/mysqldump.html#option_mysqldump_add-drop-database)).

```rust,no_run
use spatie_db_dumper::{MySql};

let dump_command = MySql::create()
    .set_user_name("username")
    .set_password("password")
    .add_extra_option("--databases")
    .add_extra_option("dbname")
    .add_extra_option("--add-drop-database")
    .get_dump_command()?;
# Ok::<(), spatie_db_dumper::Error>(())
```

With MySql, you also have the option to use the `--all-databases` extra option. This is useful when you want to run a full backup of all the databases in the specified MySQL connection.

```rust,no_run
use spatie_db_dumper::{MySql};

let dump_command = MySql::create()
    .set_user_name("username")
    .set_password("password")
    .add_extra_option("--all-databases")
    .get_dump_command()?;
# Ok::<(), spatie_db_dumper::Error>(())
```

Please note that using the `.add_extra_option("--databases").add_extra_option("dbname")` or `.add_extra_option('--all-databases')` will override the database name set on a previous `.set_db_name()` call.

### Using compression

If you want the output file to be compressed, you can use a compressor.

There are two compressors that come out of the box:

-   `GzipCompressor` - This will compress your db dump with `gzip`. Make sure `gzip` is installed on your system before using this.
-   `Bzip2Compressor` - This will compress your db dump with `bzip2`. Make sure `bzip2` is installed on your system before using this.

```rust,no_run
use spatie_db_dumper::{MySql, GzipCompressor};

let dump_command = MySql::create()
    .set_db_name("dbname")
    .set_user_name("username")
    .set_password("password")
    .use_compressor(GzipCompressor) // or `Bzip2Compressor`
    .dump_to_file("dump.sql.gz")?;
# Ok::<(), spatie_db_dumper::Error>(())
```

### Creating your own compressor

You can create your own compressor implementing the `Compressor` trait. Here's how that trait looks like:

```rust
use std::{ffi::OsString, path::PathBuf};

pub trait Compressor: Send + Sync {
    fn program(&self) -> PathBuf;
    fn args(&self) -> Vec<OsString> { Vec::new() }
    fn use_extension(&self) -> &str;
}
```

The `program` should simply return the compression executable the db dump will get pumped to. Each entry returned by `args` is one literal argument. Here's the implementation of `GzipCompressor`.

```rust
use spatie_db_dumper::Compressor;
use std::path::PathBuf;

pub struct GzipCompressor;

impl Compressor for GzipCompressor {
    fn program(&self) -> PathBuf {
        "gzip".into()
    }

    fn use_extension(&self) -> &str {
        "gz"
    }
}
```
