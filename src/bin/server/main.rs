#![allow(clippy::let_unit_value)]
use std::io::Cursor;

use chat_app::models::{Message};
use chat_app::{ChatApp, LoginToken, MessageFilter, AppError, DbError};
use rocket::futures::lock::Mutex;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::response::status::{Unauthorized};
use rocket::response::stream::EventStream;
use rocket::response::{Responder, self};
use rocket::serde::json::Json;
use rocket::{Request, Response, State};
use serde::{Deserialize, Serialize};


#[macro_use]
extern crate rocket;

#[derive(Responder, Serialize)]
#[response(content_type = "json")]
struct LoginResult {
    token: String,
}

#[derive(Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

#[launch]
fn rocket() -> _ {
    let app = match ChatApp::new() {
        Ok(app) => Mutex::new(app),
        Err(e) => {
            println!("Could not create app:\n{e}");
            std::process::exit(1)
        }
    };
    rocket::build()
        .manage(app)
        .mount("/auth", routes![login, logout])
        .mount("/", routes![send_message, get_messages, get_user, register])
}

enum RegisterResult {
    Registered,
    UsernameTaken,
    Error
}

impl<'r> Responder<'r, 'static> for RegisterResult {
    fn respond_to(self, _request: &'r Request<'_>) -> response::Result<'static> {
        match self {
            RegisterResult::Registered => Ok(Response::build().status(Status::Ok).finalize()),
            RegisterResult::UsernameTaken => Ok(Response::build().status(Status::Conflict).streamed_body(Cursor::new("Username is already taken.")).finalize()),
            RegisterResult::Error => Ok(Response::build().status(Status::InternalServerError).finalize()),
        }
    }
}

#[post("/register", data = "<credentials>")]
async fn register(app: &State<Mutex<ChatApp>>, credentials: Json<Credentials>) -> RegisterResult {
    let mut app = app.lock().await;
    match app.register(&credentials.username, &credentials.password) {
        Ok(_) => RegisterResult::Registered,
        Err(AppError::DatabaseError(DbError::UsernameInUse)) => RegisterResult::UsernameTaken,
        _ => RegisterResult::Error
    }
}

#[post("/login", data = "<login_form>")]
async fn login(
    app: &State<Mutex<ChatApp>>,
    login_form: Json<Credentials>,
) -> Result<Json<LoginResult>, Unauthorized<String>> {
    let mut app = app.lock().await;
    match app.login(&login_form.username, &login_form.password) {
        Ok(token) => Ok(Json(LoginResult { token: token.0 })),
        Err(_) => Err(Unauthorized(Some(
            "Authentication Failure. Check your credentials or try again later.".to_string(),
        ))),
    }
}

#[get("/logout")]
async fn logout(app: &State<Mutex<ChatApp>>, user: AppUser) {
    let mut app = app.lock().await;
    app.logout(&user.token);
}

#[post("/message", data = "<message>")]
async fn send_message(
    app: &State<Mutex<ChatApp>>,
    user: AppUser,
    message: &str,
) -> Result<(), Status> {
    let mut app = app.lock().await;
    match app.send_message(&user.token, message) {
        Ok(_) => Ok(()),
        _ => Err(Status::InternalServerError),
    }
}

#[get("/message", data = "<filter>")]
async fn get_messages(
    app: &State<Mutex<ChatApp>>,
    user: AppUser,
    filter: Json<MessageFilter>,
) -> Result<Json<Vec<Message>>, Status> {
    let mut app = app.lock().await;
    match app.get_messages(&user.token, &filter) {
        Ok(messages) => Ok(Json(messages)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/user", data = "<ids>")]
async fn get_user(app: &State<Mutex<ChatApp>>, ids: Json<Vec<i32>>) -> Json<Vec<Option<String>>> {
    let mut app = app.lock().await;
    let names = ids
        .iter()
        .map(|id| {
            app.get_user_by_id(*id)
                .ok()
                .map(|user| user.username)
        })
        .collect();
    Json(names)
}

struct AppUser {
    token: LoginToken,
}

#[derive(Debug)]
enum ApiKeyError {
    Missing,
    Invalid,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AppUser {
    type Error = ApiKeyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let Some(app) = req.rocket().state::<Mutex<ChatApp>>() else {
            panic!("Why the heck do we not have a app state?!")
        };

        let mut app = app.lock().await;
        let Some(header) = req.headers().get_one("Authorization") else {
            return Outcome::Failure((Status::BadRequest, ApiKeyError::Missing))
        };

        let Some(token) = header.strip_prefix("Bearer ") else {
            return Outcome::Failure((Status::BadRequest, ApiKeyError::Invalid))
        };

        let login_token = LoginToken(token.to_string());
        let Ok(_) = app.get_user_for_token(&login_token) else {
            return Outcome::Failure((Status::Forbidden, ApiKeyError::Invalid))
        };

        Outcome::Success(AppUser {
            token: login_token,
        })
    }
}
