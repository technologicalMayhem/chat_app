use std::borrow::BorrowMut;

use chat_app::models::User;
use chat_app::{ChatApp, LoginToken};
use rocket::Request;
use rocket::futures::lock::Mutex;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{response::status::Unauthorized, State};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[macro_use]
extern crate rocket;

#[derive(Debug, Error)]
enum AppError {
    #[error("No valid authentication was provided")]
    NotAuthenticated
}

#[derive(Responder, Serialize)]
#[response(content_type = "json")]
struct LoginResult {
    token: String,
}

#[derive(Deserialize)]
struct LoginForm {
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
        .mount("/auth", routes![login])
        .mount("/", routes![name])
}

#[post("/login", data = "<login_form>")]
async fn login(app: &State<Mutex<ChatApp>>,login_form: Json<LoginForm>) -> Result<Json<LoginResult>, Unauthorized<String>> {
    let mut app = app.lock().await;
    match app.login(&login_form.username, &login_form.password) {
        Ok(token) => Ok(Json(LoginResult{token: token.0})),
        Err(_) => Err(Unauthorized(Some("Authentication Failure. Check your credentials or try again later.".to_string())))
    }
}

#[get("/sayMyName")]
async fn name(user: ReqUser) -> String {
    format!("You are {}", user.0.username)
}

struct ReqUser(User);

#[derive(Debug)]
enum ApiKeyError {
    Missing,
    Invalid,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ReqUser {
    type Error = ApiKeyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self,Self::Error>  {
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
        let Ok(user) = app.get_user_for_token(&login_token) else {
            return Outcome::Failure((Status::Forbidden, ApiKeyError::Invalid))
        };

        Outcome::Success(ReqUser(user))
    }
}