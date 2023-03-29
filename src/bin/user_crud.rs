use std::{io::stdin, process::exit};

use chat_app::{
    change_username, check_password, create_user, delete_user, establish_connection, get_all_users,
    get_user_by_name, set_password,
};
use eyre::Result;
use thiserror::Error;

enum MenuOption {
    Create,
    Read,
    Update,
    Delete,
    SetPassword,
    CheckPassword,
    Exit,
}

enum ReadOption {
    Single,
    All,
}

#[derive(Error, Debug)]
enum CrudError {
    #[error("could not obtain a valid menu option")]
    InvalidMenuOption,
}

fn main() -> Result<()> {
    loop {
        let result = loop {
            let option = try_menu_main();
            if let Ok(result) = option {
                break result;
            }
        };

        let result = match result {
            MenuOption::Create => menu_create_user(),
            MenuOption::Read => menu_read_user(),
            MenuOption::Update => menu_update_user(),
            MenuOption::Delete => menu_delete_user(),
            MenuOption::Exit => exit(0),
            MenuOption::SetPassword => menu_set_password(),
            MenuOption::CheckPassword => menu_check_password(),
        };

        if let Err(e) = result {
            println!("{e}\n");
        }
    }
}

fn try_menu_main() -> Result<MenuOption> {
    println!("=== What do you want to do? ===\n");
    println!("1. Create User");
    println!("2. Read User");
    println!("3. Update User");
    println!("4. Delete User");
    println!("5. Set Password");
    println!("6. Check Password");
    println!("7. Exit");

    let int: i32 = read_string()?.parse()?;

    match int {
        1 => Ok(MenuOption::Create),
        2 => Ok(MenuOption::Read),
        3 => Ok(MenuOption::Update),
        4 => Ok(MenuOption::Delete),
        5 => Ok(MenuOption::SetPassword),
        6 => Ok(MenuOption::CheckPassword),
        7 => Ok(MenuOption::Exit),
        _ => Err(CrudError::InvalidMenuOption)?,
    }
}

fn try_menu_read() -> Result<ReadOption> {
    println!("What do you want to do?\n");
    println!("1. Read all Users");
    println!("2. Read specific User");

    let int: i32 = read_string()?.parse()?;

    match int {
        1 => Ok(ReadOption::All),
        2 => Ok(ReadOption::Single),
        _ => Err(CrudError::InvalidMenuOption)?,
    }
}

fn menu_create_user() -> Result<()> {
    println!("What name should the user have?");
    let name = read_string()?;
    let conn = &mut establish_connection()?;

    create_user(conn, &name)?;
    Ok(())
}
fn menu_read_user() -> Result<()> {
    let conn = &mut establish_connection()?;
    match try_menu_read()? {
        ReadOption::Single => {
            println!("What user should be looked up?");
            let username = read_string()?;
            let user = get_user_by_name(conn, &username)?;
            println!("\nId Name\n--------");
            println!("{}: {}", user.id, user.username);
        }
        ReadOption::All => {
            println!("\nId Name\n--------");
            for user in get_all_users(conn)? {
                println!("{}: {}", user.id, user.username);
            }
            println!();
        }
    }

    Ok(())
}

fn menu_update_user() -> Result<()> {
    let conn = &mut establish_connection()?;

    println!("Type the name of the user you want to update.");
    let cur_username = read_string()?;
    println!("Type the new username.");
    let new_username = read_string()?;

    change_username(conn, &cur_username, &new_username)?;
    println!("Sucessfully updated username!");

    Ok(())
}

fn menu_delete_user() -> Result<()> {
    let conn = &mut establish_connection()?;

    println!("Which user do you want to delete?");
    let username = read_string()?;

    delete_user(conn, &username)?;

    Ok(())
}

fn menu_set_password() -> Result<()> {
    println!("What user do you want to set a password for?");
    let username = read_string()?;
    println!("What do you want to set the password to?");
    let password = read_string()?;

    let conn = &mut establish_connection()?;
    set_password(conn, &username, &password)?;

    Ok(())
}

fn menu_check_password() -> Result<()> {
    println!("What user do you want to check the password of?");
    let username = read_string()?;
    println!("Enter their password.");
    let password = read_string()?;

    let conn = &mut establish_connection()?;
    if check_password(conn, &username, &password)? {
        println!("Correct password!");
    } else {
        println!("Incorrect password!");
    }

    Ok(())
}

fn read_string() -> Result<String> {
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;
    let mut chars = buf.chars();
    chars.next_back();

    Ok(chars.collect())
}
