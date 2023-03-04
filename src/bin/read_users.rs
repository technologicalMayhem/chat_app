use chat_app::{establish_connection, models::User};
use diesel::prelude::*;

fn main() {
    use chat_app::schema::users::dsl::*;
    let db_connection = &mut establish_connection();

    let results = users
        .limit(5)
        .load::<User>(db_connection)
        .expect("Error loading posts");

    println!("Displaying {} users", results.len());
    for user in results {
        println!("{user:?}")
    }
}