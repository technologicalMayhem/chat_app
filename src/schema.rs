// @generated automatically by Diesel CLI.

diesel::table! {
    authentications (id) {
        id -> Integer,
        userid -> Integer,
        hashedpassword -> Text,
    }
}

diesel::table! {
    messages (id) {
        id -> Integer,
        date -> Timestamp,
        messagetext -> Text,
        userid -> Integer,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        username -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(authentications, messages, users,);
