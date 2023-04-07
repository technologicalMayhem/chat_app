use chat_app::{
    models::{Credentials, LoginResult},
    LoginToken,
};
use thiserror::Error;
use ureq::Request;

#[derive(Debug, Error)]
enum Error {
    #[error("Connection failed. Check your connection or the serve address")]
    ConnectionFailure,
    #[error("Failed to login. Check your credentials or try again later.")]
    LoginFailed,
    #[error("Failed to deserialize data received from the server. This is a bug.")]
    DeserializingFailed,
    #[error("Could not register. The username is already in use.")]
    UsernameInUse,
    #[error("Could not register. A unknown error occured.")]
    RegistrationFailed,
}

const STATUS_CONFLICT: u16 = 409;

struct Client {
    token: LoginToken,
    address: String
}

impl Client {
    fn login(address: &str, username: &str, password: &str) -> Result<Self, Error> {
        let credentials = Credentials {
            username: username.to_string(),
            password: password.to_string(),
        };
        let response = match ureq::post(&format!("{address}/auth/login")).send_json(credentials) {
            Ok(response) => response,
            Err(e) => match e {
                ureq::Error::Status(_, _) => return Err(Error::LoginFailed),
                ureq::Error::Transport(_) => return Err(Error::ConnectionFailure),
            },
        };

        let login: LoginResult = response.into_json().map_err(|_| Error::DeserializingFailed)?;
        Ok(Self {
            token: LoginToken(login.token),
            address: address.to_string(),
        })
    }

    fn register(address: &str, username: &str, password: &str) -> Result<Self, Error> {
        let credentials = Credentials {
            username: username.to_string(),
            password: password.to_string(),
        };
        let response = match ureq::post(&format!("{address}/register")).send_json(&credentials) {
            Ok(response) => response,
            Err(e) => match e {
                ureq::Error::Status(STATUS_CONFLICT, _) => return Err(Error::UsernameInUse),
                ureq::Error::Status(_, _) => return Err(Error::RegistrationFailed),
                ureq::Error::Transport(_) => return Err(Error::ConnectionFailure),
            },
        };

        Self::login(address, username, password)
    }

    fn logout(&self) -> Result<(), Error> {
        match ureq::get(&format!("{}/auth/logout", self.address)).auth(&self).call() {
            Ok(response) => response,
            Err(_) => return Err(Error::ConnectionFailure),
        };

        Ok(())
    }
}

trait AttachHeader {
    fn auth(self, client: &Client) -> Self;
}

impl AttachHeader for Request {
    fn auth(self, client: &Client) -> Self {
        self.set("Authorization",  &format!("Bearer {}", client.token.0))
    }
}