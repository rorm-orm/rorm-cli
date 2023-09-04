use std::cmp::Ordering;
use std::path::Path;

use anyhow::{anyhow, Context};
use rorm_db::executor::{Executor, Nothing, Optional};
use rorm_db::Database;
use rorm_declaration::config::{DatabaseConfig, DatabaseDriver};
use rorm_declaration::imr::{Annotation, DbType};
use rorm_declaration::migration::Migration;
use rorm_sql::create_table::CreateTable;
use rorm_sql::insert::Insert;
use rorm_sql::DBImpl;

use crate::log_sql;
use crate::migrate::config::{create_db_config, deserialize_db_conf};
use crate::migrate::sql_builder::migration_to_sql;
use crate::utils::migrations::get_existing_migrations;

pub mod config;
pub mod sql_builder;

/**
Options for running migrations
*/
pub struct MigrateOptions {
    /// Directory, migrations exist in
    pub migration_dir: String,

    /// Path to the database configuration file
    pub database_config: String,

    /// Log all SQL statements
    pub log_queries: bool,

    /// Apply only to (inclusive) the given id, if set
    pub apply_until: Option<u16>,
}

/**
Helper method to apply one migration. Writes also to last migration table.

`migration`: [&Migration]: Reference to the migration to apply.
`pool`: [&SqlitePool]: Pool to apply the migration onto.
`last_migration_table_name`: [&str]: Name of the table to insert successful applied migrations into.
*/
pub async fn apply_migration(
    dialect: DBImpl,
    migration: &Migration,
    pool: &Database,
    last_migration_table_name: &str,
    do_log: bool,
) -> anyhow::Result<()> {
    let mut tx = pool
        .start_transaction()
        .await
        .with_context(|| format!("Error while starting transaction {}", migration.id))?;

    if let Err(e) = migration_to_sql(&mut tx, dialect, migration, do_log).await {
        tx.rollback()
            .await
            .with_context(|| "Error while rollback in transaction")?;
        return Err(e);
    }

    let v: &[&[rorm_sql::value::Value]] = &[&[rorm_sql::value::Value::I32(migration.id as i32)]];
    let (query_string, bind_params) = dialect
        .insert(last_migration_table_name, &["migration_id"], v, None)
        .rollback_transaction()
        .build();

    if do_log {
        println!("{query_string}");
    }

    pool.execute::<Nothing>(query_string, bind_params).await.with_context(|| {
        format!(
            "Error while inserting applied migration {last_migration_table_name} into last migration table",
        )
    })?;

    println!("Applied migration {:04}_{}", migration.id, migration.name);

    tx.commit().await.with_context(|| {
        format!("Error while committing transaction {last_migration_table_name}",)
    })?;

    Ok(())
}

/// Applies migrations on the given database with a given driver
pub async fn run_migrate_custom(
    db_conf: DatabaseConfig,
    migration_dir: String,
    log_sql: bool,
    apply_until: Option<u16>,
) -> anyhow::Result<()> {
    let p = Path::new(migration_dir.as_str());
    if !p.exists() || p.is_file() {
        println!(
            "Couldn't find the migration directory in {} \n\n\
            You can specify an alternative path with --migration-dir <PATH>",
            migration_dir.as_str()
        );
        return Ok(());
    }

    let existing_migrations = get_existing_migrations(migration_dir.as_str())
        .with_context(|| "Couldn't retrieve existing migrations")?;

    if existing_migrations.is_empty() {
        println!("No migrations found.\nExiting.");
        return Ok(());
    }

    let pool = Database::connect(rorm_db::DatabaseConfiguration {
        driver: db_conf.driver,
        min_connections: 1,
        max_connections: 1,
        disable_logging: None,
        statement_log_level: None,
        slow_statement_log_level: None,
    })
    .await?;

    let last_migration_table_name = db_conf
        .last_migration_table_name
        .as_ref()
        .map_or("_rorm__last_migration", |x| x.as_str());

    let db_impl = (&pool).dialect();
    let statements = db_impl
        .create_table(last_migration_table_name)
        .add_column(db_impl.create_column(
            last_migration_table_name,
            "id",
            DbType::Int64,
            &[Annotation::PrimaryKey, Annotation::AutoIncrement],
        ))
        .add_column(db_impl.create_column(
            last_migration_table_name,
            "updated_at",
            DbType::DateTime,
            &[Annotation::AutoUpdateTime],
        ))
        .add_column(db_impl.create_column(
            last_migration_table_name,
            "migration_id",
            DbType::Int32,
            &[Annotation::NotNull],
        ))
        .if_not_exists()
        .build()?;

    let mut tx = pool
        .start_transaction()
        .await
        .with_context(|| "Could not create transaction")?;

    for (query_string, bind_params) in statements {
        if log_sql {
            println!("{}", query_string.as_str());
        }

        tx.execute::<Nothing>(query_string, bind_params)
            .await
            .with_context(|| "Couldn't create internal last migration table")?;
    }

    tx.commit()
        .await
        .with_context(|| "Couldn't create internal last migration table")?;

    let last_migration: Option<i32> = pool
        .execute::<Optional>(
            log_sql!(
                format!(
                    "SELECT migration_id FROM {} ORDER BY id DESC LIMIT 1;",
                    &last_migration_table_name
                ),
                log_sql
            ),
            Vec::new(),
        )
        .await
        .and_then(|option| option.map(|row| row.get(0)).transpose())
        .with_context(|| {
            "Couldn't fetch information about successful migrations from migration table"
        })?;

    match last_migration {
        None => {
            // Apply all migrations
            for migration in &existing_migrations {
                apply_migration(
                    db_impl,
                    migration,
                    &pool,
                    last_migration_table_name,
                    log_sql,
                )
                .await?;

                if let Some(apply_until) = apply_until {
                    if migration.id == apply_until {
                        println!(
                            "Applied all migrations until (inclusive) migration {apply_until:04}"
                        );
                        break;
                    }
                }
            }
        }
        Some(id) => {
            let id = id as u16;
            // Search for last applied migration
            if existing_migrations.iter().any(|x| x.id == id) {
                let mut apply = false;
                for (idx, migration) in existing_migrations.iter().enumerate() {
                    if apply {
                        apply_migration(
                            db_impl,
                            migration,
                            &pool,
                            last_migration_table_name,
                            log_sql,
                        )
                        .await?;
                        continue;
                    }

                    if migration.id == id {
                        apply = true;

                        if idx == existing_migrations.len() - 1 {
                            println!("All migration have already been applied.");
                        }
                    }

                    if let Some(apply_until) = apply_until {
                        match migration.id.cmp(&apply_until) {
                            Ordering::Equal => {
                                if apply {
                                    println!(
                                        "Applied all migrations until (inclusive) migration {apply_until:04}"
                                    );
                                } else {
                                    println!(
                                        "All migrations until (inclusive) migration {apply_until:04} have already been applied"
                                    );
                                }
                                break;
                            }
                            Ordering::Greater => break,
                            Ordering::Less => {}
                        }
                    }
                }
            } else {
                // If last applied migration could not be found in existing migrations,
                // panic as there's no way to determine what to do next
                return Err(anyhow!(
                    r#"Last applied migration {id} was not found in current migrations.
 
Can not proceed any further without damaging data.
To correct, empty the {last_migration_table_name} table or reset the whole database."#,
                ));
            }
        }
    }

    Ok(())
}

/// Applies migrations on the given database
pub async fn run_migrate(options: MigrateOptions) -> anyhow::Result<()> {
    let db_conf_path = Path::new(options.database_config.as_str());

    if !&db_conf_path.exists() {
        println!(
            "Couldn't find the database configuration file, created {} and exiting",
            options.database_config.as_str()
        );
        create_db_config(db_conf_path)?;
        return Ok(());
    }

    let db_conf = deserialize_db_conf(db_conf_path)?;

    if let DatabaseDriver::SQLite { filename } = &db_conf.driver {
        if filename.is_empty() {
            println!("Invalid configuration: Filename for sqlite is empty");
            return Ok(());
        }
    }

    run_migrate_custom(
        db_conf,
        options.migration_dir,
        options.log_queries,
        options.apply_until,
    )
    .await
}
