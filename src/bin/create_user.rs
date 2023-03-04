use std::io::stdin;

use chat_app::*;

fn main() {
    let connection = &mut establish_connection();

    let mut name = String::new();
    println!("Give a name for a new user");
    stdin().read_line(&mut name).unwrap();
    let name = name.trim_end(); // Remove the trailing newline

    create_user(connection, &name);

    println!("Sucesfully created user!");
}