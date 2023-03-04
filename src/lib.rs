use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use crate::models::{NewUser};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub mod models;
pub mod schema;

pub fn create_user(conn: &mut SqliteConnection, username: &str) -> usize {
    use crate::schema::users;
    let new_user = NewUser { username };

    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .expect("Error creating new user")
}

pub fn establish_connection() -> SqliteConnection {
    let mut connection = SqliteConnection::establish("data.db")
        .unwrap_or_else(|_| panic!("Error connecting to database."));
    connection.run_pending_migrations(MIGRATIONS).expect("");

    connection
}
