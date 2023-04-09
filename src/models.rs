use crate::schema::{authentications, messages, users};
use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable, Selectable};
use rocket::response::Responder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Queryable, Selectable, Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
}

#[derive(Debug, Queryable)]
pub struct Authentication {
    pub id: i32,
    pub userid: i32,
    pub hashedpassword: String,
}

#[derive(Debug, Queryable, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: i32,
    pub date: NaiveDateTime,
    pub messagetext: String,
    pub userid: i32,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub username: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = authentications)]
pub struct NewAuthentication {
    pub userid: i32,
    pub hashedpassword: String,
}

#[derive(Insertable)]
#[diesel(table_name = messages)]
pub struct NewMessage {
    pub date: NaiveDateTime,
    pub messagetext: String,
    pub userid: i32,
}

#[derive(Responder, Serialize, Deserialize)]
#[response(content_type = "json")]
pub struct LoginResult {
    pub token: String,
}

#[derive(Deserialize, Serialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}
