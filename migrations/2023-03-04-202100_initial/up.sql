-- Your SQL goes here
CREATE TABLE users (
    id INTEGER NOT NULL PRIMARY KEY,
    username TEXT NOT NULL
);

CREATE TABLE authentications (
    id INTEGER NOT NULL PRIMARY KEY,
    userid INTEGER NOT NULL,
    hashedpassword TEXT NOT NULL,
    FOREIGN KEY(userid) REFERENCES users(id)
);

CREATE TABLE messages (
    id INTEGER NOT NULL PRIMARY KEY,
    date TIMESTAMP NOT NULL,
    messagetext TEXT NOT NULL,
    userid INTEGER NOT NULL,
    FOREIGN KEY(userid) REFERENCES users(id)
);
