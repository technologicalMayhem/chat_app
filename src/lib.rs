use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use schema::users;
use thiserror::Error;

use crate::models::{NewUser, User};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub mod models;
pub mod schema;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Could not connect to database")]
    ConnectionFailure,
    #[error("Could not apply database migrations")]
    MigrationFailure,
    #[error("Could not insert user into database")]
    UserCreationFailed,
    #[error("A user with that name already exists")]
    UsernameInUse,
    #[error("Could not lookup user in database")]
    UserFilterFailed,
    #[error("Found multiple users with same username")]
    UsernameCollisionDetected,
    #[error("Could not find a user with that name")]
    UserNotFound,
    #[error("The underlying database engine encountered an error")]
    GenericError(#[from] diesel::result::Error),
}

/// Create a new user.
///
/// # Errors
///
/// This function will return an error if the cration of the user failed.
pub fn create_user(conn: &mut SqliteConnection, name: &str) -> Result<(), DbError> {
    if get_user(conn, name).is_ok() {
        return Err(DbError::UsernameInUse);
    }

    let new_user = NewUser { username: name };

    match diesel::insert_into(users::table)
        .values(&new_user)
        .execute(conn)
    {
        Ok(_) => Ok(()),
        Err(_) => Err(DbError::UserCreationFailed)?,
    }
}

/// Get a specific user from the database.
///
/// # Errors
///
/// This function will return an error if no user with that name could be found.
pub fn get_user(conn: &mut SqliteConnection, name: &str) -> Result<User, DbError> {
    use crate::schema::users::dsl::{username, users};

    let Ok(mut found_users) = users.filter(username.eq(name)).load::<User>(conn) else { return Err(DbError::UserFilterFailed)? };

    if found_users.len() > 1 {
        Err(DbError::UsernameCollisionDetected)?
    } else {
        let Some(user) = found_users.pop() else {return Err(DbError::UserNotFound);};

        Ok(user)
    }
}

/// Change the name of a user.
///
/// # Errors
///
/// This function will return an error if no user with that name could be found or it would cause a collision.
pub fn change_username(
    conn: &mut SqliteConnection,
    current_username: &str,
    new_username: &str,
) -> Result<(), DbError> {
    use crate::schema::users::dsl::{username, users};

    if get_user(conn, new_username).is_ok() {
        return Err(DbError::UsernameInUse);
    }

    let user_to_update = users.filter(username.eq(current_username));
    let rows_affected = diesel::update(user_to_update)
        .set(username.eq(new_username))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(DbError::UserNotFound);
    }

    Ok(())
}

/// Delete a user.
///
/// # Errors
///
/// This function will return an error if the operation fails.
pub fn delete_user(conn: &mut SqliteConnection, name: &str) -> Result<(), DbError> {
    use crate::schema::users::dsl::{username, users};

    let user_to_delete = users.filter(username.eq(name));
    let affected_rows = diesel::delete(user_to_delete).execute(conn)?;

    if affected_rows == 0 {
        return Err(DbError::UserNotFound);
    }

    Ok(())
}

/// Returns all users.
///
/// # Errors
///
/// This function will return an error if reading all entries from the user table fails.
pub fn get_all_users(conn: &mut SqliteConnection) -> Result<Vec<User>, DbError> {
    Ok(users::dsl::users.load::<User>(conn)?)
}

/// Establish a connection to the database.
///
/// # Errors
///
/// This function will return an error if a connection could not be established or the database schema is not valid.
pub fn establish_connection() -> Result<SqliteConnection, DbError> {
    let mut connection =
        SqliteConnection::establish("data.db").or(Err(DbError::UsernameCollisionDetected))?;
    connection
        .run_pending_migrations(MIGRATIONS)
        .or(Err(DbError::MigrationFailure))?;

    Ok(connection)
}
