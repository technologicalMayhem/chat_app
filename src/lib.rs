use std::time::{Duration, SystemTime};

use base64::Engine;
use chrono::{DateTime, Local};
use diesel::r2d2::ConnectionManager;
use diesel::{prelude::*, r2d2::Pool};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use models::{Message, NewMessage};
use rand::Rng;
use thiserror::Error;

use crate::models::{Authentication, NewAuthentication, NewUser, User};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

mod auth;
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
    #[error("Failed to create connection pool")]
    PoolError(#[from] r2d2::Error),
    #[error("No password set")]
    NoPasswordSet,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("A database operation caused an error")]
    DatabaseError(#[from] DbError),
    #[error("Failed to obtain a connection from the connection pool")]
    PoolError(#[from] r2d2::Error),
    #[error("Failed to login user")]
    LoginFailed,
    #[error("The given token is invalid")]
    TokenInvalid,
}

pub struct ChatApp {
    db_connection: Pool<ConnectionManager<SqliteConnection>>,
    active_logins: Vec<ActiveLogin>,
}

impl ChatApp {
    /// Create a new `ChatApp` instance using a local Sqlite database store.
    ///
    /// # Errors
    ///
    /// This function will return an error if connecting to the database fails.
    pub fn new() -> Result<Self, AppError> {
        Ok(ChatApp {
            db_connection: get_connection_pool()?,
            active_logins: Vec::new(),
        })
    }

    /// Register a new user.
    ///
    /// # Errors
    ///
    /// This function will return an error if registering the user failed.
    pub fn register(&mut self, username: &str, password: &str) -> Result<(), AppError> {
        let conn = &mut self.db_connection.get()?;
        create_user(conn, username)?;
        set_password(conn, username, password)?;
        Ok(())
    }

    /// Login as the user, returning a `LoginToken` for further operations.
    ///
    /// # Errors
    ///
    /// This function will return an error if the authentication failed.
    pub fn login(&mut self, username: &str, password: &str) -> Result<LoginToken, AppError> {
        let conn = &mut &mut self.db_connection.get()?;
        if check_password(conn, username, password)? {
            let active_login = ActiveLogin::new(username);
            let login_token = active_login.token.clone();

            self.active_logins.push(active_login);

            Ok(login_token)
        } else {
            Err(AppError::LoginFailed)
        }
    }

    /// Logout the user, invalidating the token.
    pub fn logout(&mut self, login_token: &LoginToken) {
        for (index, login) in self.active_logins.iter().enumerate() {
            if login.token == *login_token {
                self.active_logins.remove(index);
                break;
            }
        }
    }

    /// Send a message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the login token is not valid or the messaged could not be sent.
    pub fn send_message(
        &mut self,
        login_token: &LoginToken,
        message: &str,
    ) -> Result<(), AppError> {
        let user = self.get_user_for_token(login_token)?;
        let conn = &mut &mut self.db_connection.get()?;
        create_message(conn, message, user.id)?;

        Ok(())
    }

    /// Get the messages to show the user.
    ///
    /// # Errors
    ///
    /// This function will return an error if the user is not logged in or the messages could not be retrieved.
    pub fn get_messages(
        &mut self,
        login_token: &LoginToken,
        filter: &MessageFilter,
    ) -> Result<Vec<Message>, AppError> {
        if self.get_username_for_token(login_token).is_none() {
            return Err(AppError::TokenInvalid);
        }
        let conn = &mut &mut self.db_connection.get()?;
        Ok(get_messages(conn, filter)?)
    }

    /// Gets the user with that id.
    ///
    /// # Errors
    ///
    /// This function will return an error if the user does not exist.
    pub fn get_user_by_id(&mut self, id: i32) -> Result<User, AppError> {
        let conn = &mut &mut self.db_connection.get()?;
        Ok(get_user_by_id(conn, id)?)
    }

    fn get_user_for_token(&mut self, login_token: &LoginToken) -> Result<User, AppError> {
        let Some(username) = self.get_username_for_token(login_token) else {return Err(AppError::TokenInvalid)};
        let conn = &mut &mut self.db_connection.get()?;
        Ok(get_user_by_name(conn, &username)?)
    }

    fn get_username_for_token(&mut self, login_token: &LoginToken) -> Option<String> {
        let mut found = None;
        let mut to_prune: Vec<usize> = Vec::new();
        let now = SystemTime::now();
        for (index, login) in self.active_logins.iter().enumerate() {
            if login.valid_until < now {
                to_prune.push(index);
                continue;
            }
            if login.token != *login_token {
                continue;
            }
            found = Some(login.username.clone());
        }

        for index in to_prune {
            self.active_logins.remove(index);
        }

        found
    }
}

struct ActiveLogin {
    username: String,
    token: LoginToken,
    valid_until: SystemTime,
}

impl ActiveLogin {
    pub fn new(username: &str) -> Self {
        let username = username.into();

        let mut rng = rand::thread_rng();
        let data: Vec<u8> = (1..8).map(|_| rng.gen()).collect();
        let encoded_data = base64::engine::general_purpose::STANDARD_NO_PAD.encode(data);
        let token = LoginToken(encoded_data);

        let valid_until = SystemTime::now() + Duration::from_secs(1200); // Valid for 20 minutes

        ActiveLogin {
            username,
            token,
            valid_until,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginToken(pub String);

/// Create a new user.
///
/// # Errors
///
/// This function will return an error if the cration of the user failed.
pub fn create_user(conn: &mut SqliteConnection, name: &str) -> Result<(), DbError> {
    if get_user_by_name(conn, name).is_ok() {
        return Err(DbError::UsernameInUse);
    }

    let new_user = NewUser { username: name };

    match diesel::insert_into(schema::users::table)
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
pub fn get_user_by_name(conn: &mut SqliteConnection, name: &str) -> Result<User, DbError> {
    use crate::schema::users::dsl::{username, users};

    let Ok(mut found_users) = users.filter(username.eq(name)).load::<User>(conn) else { return Err(DbError::UserFilterFailed)? };

    if found_users.len() > 1 {
        Err(DbError::UsernameCollisionDetected)?
    } else {
        let Some(user) = found_users.pop() else {return Err(DbError::UserNotFound);};

        Ok(user)
    }
}

    /// Gets the user with that id.
    ///
    /// # Errors
    ///
    /// This function will return an error if the user does not exist.
pub fn get_user_by_id(conn: &mut SqliteConnection, id: i32) -> Result<User, DbError> {
    use crate::schema::users::dsl::{id as user_id, users};

    Ok(users.filter(user_id.eq(id)).first::<User>(conn)?)
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

    if get_user_by_name(conn, new_username).is_ok() {
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
    Ok(schema::users::dsl::users.load::<User>(conn)?)
}

/// Sets the password for the given user.
///
/// # Errors
///
/// This function will return an error if the user does not exist or the password could not be set.
pub fn set_password(
    conn: &mut SqliteConnection,
    username: &str,
    password: &str,
) -> Result<(), DbError> {
    use schema::authentications::dsl::{authentications, hashedpassword, userid};
    let hash = auth::generate_hash(password);
    let user = get_user_by_name(conn, username)?;
    let user_auth_data = authentications.filter(userid.eq(user.id));
    let auth_exists = user_auth_data.first::<Authentication>(conn).is_ok();

    if auth_exists {
        diesel::update(user_auth_data)
            .set(hashedpassword.eq(hash))
            .execute(conn)?;
    } else {
        let auth_data = NewAuthentication {
            userid: user.id,
            hashedpassword: hash,
        };
        diesel::insert_into(authentications)
            .values(auth_data)
            .execute(conn)?;
    }

    Ok(())
}

/// Checks if the given username and password are valid.
///
/// # Errors
///
/// This function will return an error if the user does not exist or no password is set.
pub fn check_password(
    conn: &mut SqliteConnection,
    username: &str,
    password: &str,
) -> Result<bool, DbError> {
    use schema::authentications::dsl::{authentications, userid};
    let user = get_user_by_name(conn, username)?;
    let Ok(auth_data) = authentications.filter(userid.eq(user.id)).first::<Authentication>(conn) else {
        return Err(DbError::NoPasswordSet);
    };

    Ok(auth::verify_password(password, &auth_data.hashedpassword))
}

/// Creates a new message.
///
/// # Errors
///
/// This function will return an error if inserting the message into the database fails.
pub fn create_message(
    conn: &mut SqliteConnection,
    message: &str,
    userid: i32,
) -> Result<(), DbError> {
    let date = Local::now();
    let new_message = NewMessage {
        date: date.to_rfc3339(),
        messagetext: message.into(),
        userid,
    };
    diesel::insert_into(schema::messages::table)
        .values(new_message)
        .execute(conn)?;

    Ok(())
}

pub enum MessageFilter {
    Before(DateTime<Local>),
    After(DateTime<Local>),
}

/// Get messages written before or after the given date, lmited to 20 at a time.
///
/// # Errors
///
/// This function will return an error if the messages cannot be retrieved.
pub fn get_messages(
    conn: &mut SqliteConnection,
    filter: &MessageFilter,
) -> Result<Vec<Message>, DbError> {
    use schema::messages::dsl::{date, messages};
    let query = messages.order_by(date).limit(20);

    let result = match filter {
        MessageFilter::Before(before) => query
            .filter(date.lt(before.to_rfc3339()))
            .load::<Message>(conn)?,
        MessageFilter::After(after) => query
            .filter(date.gt(after.to_rfc3339()))
            .load::<Message>(conn)?,
    };

    Ok(result)
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

pub fn get_connection_pool() -> Result<Pool<ConnectionManager<SqliteConnection>>, DbError> {
    let url = "data.db";
    let manager = ConnectionManager::<SqliteConnection>::new(url);
    // Refer to the `r2d2` documentation for more methods to use
    // when building a connection pool
    match Pool::builder()
            .test_on_check_out(true)
            .build(manager) {
        Ok(pool) => Ok(pool),
        Err(_) => {Err(DbError::ConnectionFailure)},
    }
}
