use crate::schema::{authentications, messages, users};
use diesel::{Insertable, Queryable, Selectable};
use serde::Serialize;

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

#[derive(Debug, Queryable, Serialize, Clone)]
pub struct Message {
    pub id: i32,
    pub date: String,
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
    pub date: String,
    pub messagetext: String,
    pub userid: i32,
}
