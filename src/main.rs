pub mod init;
pub mod linter;
pub mod make_migrations;
pub mod merge_migrations;
pub mod migrate;
pub mod squash_migrations;
pub mod utils;

use crate::init::init;
use clap::{ArgAction, Parser, Subcommand};

use crate::make_migrations::{run_make_migrations, MakeMigrationsOptions};
use crate::migrate::{run_migrate, MigrateOptions};
use crate::squash_migrations::squash_migrations;

#[derive(Subcommand)]
pub enum InitDriver {
    #[clap(about = "Initialize a sqlite configuration")]
    Sqlite {
        #[clap(long = "filename")]
        #[clap(default_value_t = String::from("db.sqlite3"))]
        #[clap(help = "Name of the sqlite file.")]
        filename: String,
    },
    #[clap(about = "Initialize a mysql configuration")]
    Mysql {
        #[clap(long = "host")]
        #[clap(default_value_t = String::from("127.0.0.1"))]
        #[clap(help = "The address to use to connect to the database.")]
        host: String,
        #[clap(long = "port")]
        #[clap(default_value_t = 5432)]
        #[clap(help = "The port to use to connect to the database.")]
        port: u16,
        #[clap(long = "user")]
        #[clap(default_value_t = String::from("dbuser"))]
        #[clap(help = "The user to use to connect to the database.")]
        user: String,
        #[clap(long = "password")]
        #[clap(
            help = "Set the password. To minimize the risk of exposing your password, use --ask-password instead."
        )]
        password: Option<String>,
        #[clap(long = "ask-password")]
        #[clap(
            help = "Ask for the password for the database. If specified with the --password option, this value will be prevalent."
        )]
        ask_password: bool,
        #[clap(long = "name")]
        #[clap(default_value_t = String::from("dbname"))]
        #[clap(help = "The name of the database to connect to.")]
        name: String,
    },
    #[clap(about = "Initialize a postgres configuration")]
    Postgres {
        #[clap(long = "host")]
        #[clap(default_value_t = String::from("127.0.0.1"))]
        #[clap(help = "The address to use to connect to the database.")]
        host: String,
        #[clap(long = "port")]
        #[clap(default_value_t = 5432)]
        #[clap(help = "The port to use to connect to the database.")]
        port: u16,
        #[clap(long = "user")]
        #[clap(default_value_t = String::from("dbuser"))]
        #[clap(help = "The user to use to connect to the database.")]
        user: String,
        #[clap(long = "password")]
        #[clap(
            help = "Set the password. To minimize the risk of exposing your password, use --ask-password instead."
        )]
        password: Option<String>,
        #[clap(long = "ask-password")]
        #[clap(
            help = "Ask for the password for the database. If specified with the --password option, this value will be prevalent."
        )]
        ask_password: bool,
        #[clap(long = "name")]
        #[clap(default_value_t = String::from("dbname"))]
        #[clap(help = "The name of the database to connect to.")]
        name: String,
    },
}

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "Create the database configuration file")]
    Init {
        #[clap(long = "database-config")]
        #[clap(default_value_t = String::from("./database.toml"))]
        #[clap(help = "Path to the database configuration file that should be created.")]
        database_config: String,

        #[clap(short = 'f', long = "force")]
        #[clap(help = "Overwrite the database configuration if it is existent already")]
        force: bool,

        #[clap(subcommand)]
        driver: InitDriver,
    },

    #[clap(about = "Tool to create migrations")]
    MakeMigrations {
        #[clap(long = "models-file")]
        #[clap(default_value_t=String::from("./.models.json"))]
        #[clap(help = "Location of the intermediate representation of models.")]
        models_file: String,

        #[clap(short = 'm', long = "migration-dir")]
        #[clap(default_value_t=String::from("./migrations/"))]
        #[clap(help = "Destination to / from which migrations are written / read.")]
        migration_dir: String,

        #[clap(help = "Use this name as migration name instead of generating one.")]
        name: Option<String>,

        #[clap(long = "non-interactive")]
        #[clap(action = ArgAction::SetTrue)]
        #[clap(help = "If set, no questions will be asked.")]
        non_interactive: bool,

        #[clap(long = "disable-warnings")]
        #[clap(action = ArgAction::SetTrue)]
        #[clap(help = "If set, no warnings will be printed.")]
        warnings_disabled: bool,
    },

    #[clap(about = "Apply migrations")]
    Migrate {
        #[clap(short = 'm', long = "migration-dir")]
        #[clap(default_value_t=String::from("./migrations/"))]
        #[clap(help = "Destination to / from which migrations are written / read.")]
        migration_dir: String,

        #[clap(long = "database-config")]
        #[clap(default_value_t=String::from("./database.toml"))]
        #[clap(help = "Path to the database configuration file.")]
        database_config: String,

        #[clap(long = "log-sql")]
        #[clap(action = ArgAction::SetTrue)]
        #[clap(help = "If turned on, all queries to the database will be logged")]
        log_queries: bool,

        #[clap(long = "apply-until")]
        #[clap(id = "MIGRATION_ID")]
        #[clap(help = "Only apply the migrations to (inclusive) the given migration.")]
        apply_until: Option<u16>,
    },

    #[clap(about = "Squash migrations")]
    SquashMigrations {
        #[clap(short = 'm', long = "migration-dir")]
        #[clap(default_value_t = String::from("./migrations/"))]
        #[clap(help = "Destination to / from which migrations are written / read.")]
        migration_dir: String,

        #[clap(help = "First migration to start squashing from.")]
        first_migration: u16,

        #[clap(help = "Last migration to squash.")]
        last_migration: u16,
    },

    #[clap(about = "Merge migrations")]
    MergeMigrations {},
}

#[derive(Parser)]
#[clap(version = "0.1.0", about = "CLI tool for rorm", long_about = None)]
#[clap(arg_required_else_help = true)]
#[clap(name = "rorm-cli")]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();

    match cli.command {
        Some(Commands::Init {
            force,
            driver,
            database_config,
        }) => init(database_config, driver, force)?,
        Some(Commands::MakeMigrations {
            models_file,
            migration_dir,
            name,
            non_interactive,
            warnings_disabled,
        }) => {
            run_make_migrations(MakeMigrationsOptions {
                models_file,
                migration_dir,
                name,
                non_interactive,
                warnings_disabled,
            })?;
        }
        Some(Commands::Migrate {
            migration_dir,
            database_config,
            log_queries,
            apply_until,
        }) => {
            run_migrate(MigrateOptions {
                migration_dir,
                database_config,
                log_queries,
                apply_until,
            })
            .await?;
        }
        Some(Commands::SquashMigrations {
            migration_dir,
            first_migration,
            last_migration,
        }) => {
            squash_migrations(migration_dir, first_migration, last_migration).await?;
        }
        _ => {}
    }
    Ok(())
}
