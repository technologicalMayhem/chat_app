-- Your SQL goes here
CREATE TABLE users (
    id INTEGER NOT NULL PRIMARY KEY,
    username TEXT NOT NULL
);

CREATE TABLE authentications (
    id INTEGER NOT NULL PRIMARY KEY,
    userid INTEGER NOT NULL,
    salt TEXT NOT NULL,
    hashedpassword TEXT NOT NULL,
    FOREIGN KEY(userid) REFERENCES user(userid)
);

CREATE TABLE messages (
    id INTEGER NOT NULL PRIMARY KEY,
    date DATE NOT NULL,
    messagetext TEXT NOT NULL,
    userid INTEGER NOT NULL,
    FOREIGN KEY(userid) REFERENCES user(userid)
);
