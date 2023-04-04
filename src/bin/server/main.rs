use std::borrow::BorrowMut;

use chat_app::{ChatApp, LoginToken};
use rocket::futures::lock::Mutex;
use rocket::{response::status::Unauthorized, State};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

#[macro_use]
extern crate rocket;

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
        .mount("/hello", routes![world])
        .mount("/auth", routes![login])
}

#[get("/world")]
fn world() -> &'static str {
    "Hello, world!"
}

#[post("/login", data = "<login_form>")]
async fn login(app: &State<Mutex<ChatApp>>,login_form: Json<LoginForm>) -> Result<Json<LoginResult>, Unauthorized<String>> {
    let mut app = app.lock().await;
    match app.login(&login_form.username, &login_form.password) {
        Ok(token) => Ok(Json(LoginResult{token: token.0})),
        Err(_) => Err(Unauthorized(Some("Authentication Failure. Check your credentials or try again later.".to_string())))
    }
}
