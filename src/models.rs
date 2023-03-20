use crate::schema::{authentications, users};
use diesel::{Insertable, Queryable, Selectable};

#[derive(Debug, Queryable, Selectable)]
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

#[derive(Debug, Queryable)]
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
