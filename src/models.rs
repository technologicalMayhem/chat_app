use diesel::{Queryable, Insertable};
use crate::schema::users;

#[derive(Debug, Queryable)]
pub struct User {
    pub id: i32,
    pub username: String
}

#[derive(Debug, Queryable)]
pub struct Authentication {
    pub id: i32,
    pub user_id: i32,
    pub salt: String,
    pub hashed_password: String,
}

#[derive(Debug, Queryable)]
pub struct Message {
    pub id: i32,
    pub date: String,
    pub message_text: String,
    pub user_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub username: &'a str,
}