use crate::schema::users;
use diesel::{Insertable, Queryable, Selectable};

#[derive(Debug, Queryable, Selectable)]
pub struct User {
    pub id: i32,
    pub username: String,
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
