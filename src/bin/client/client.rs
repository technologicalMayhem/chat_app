use std::{
    collections::HashMap,
    sync::mpsc::Receiver,
};

use chat_app::{
    models::{Credentials, LoginResult, Message},
    LoginToken, MessageFilter,
};
use reqwest::{Client as HttpClient, RequestBuilder, StatusCode};
use reqwest_eventsource::{EventSource, Event};
use rocket::futures::StreamExt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Could not create HTTP client.")]
    ClientCreationFailed(reqwest::Error),
    #[error("Could not create EventSource handler.")]
    EventSourceCreationFailed(reqwest_eventsource::CannotCloneRequestError),
    #[error("The server returned a unexpected status {code} when acessing the {endpoint} endpoint. This is a server bug.")]
    UnexpectedStatusCode { code: StatusCode, endpoint: String },
    #[error("The server returned a invalid or malformed response. This is a server bug.")]
    InvalidRespone(reqwest::Error),
    #[error("A error occured whilst performing a request to the server.")]
    Generic(reqwest::Error),
    #[error("Connection failed. Check your connection or the server address")]
    ConnectionFailure(reqwest::Error),
    #[error("Authentication failed. Login again and try again.")]
    NotAuthorized,
    #[error("Failed to login. Check your credentials or try again later.")]
    LoginFailed,
    #[error("Failed to deserialize data received from the server. This is a bug.")]
    DeserializingFailed(reqwest::Error),
    #[error("Could not register. The username is already in use.")]
    UsernameInUse
}

pub struct Client {
    token: LoginToken,
    address: String,
    http_client: HttpClient,
}

impl Client {
    pub async fn login(address: &str, username: &str, password: &str) -> Result<Self, Error> {
        let credentials = Credentials {
            username: username.to_string(),
            password: password.to_string(),
        };
        let client = Self::create_client()?;
        Self::inner_login(address, credentials, client).await
    }

    pub async fn register(address: &str, username: &str, password: &str) -> Result<Self, Error> {
        let credentials = Credentials {
            username: username.to_string(),
            password: password.to_string(),
        };
        let client = Self::create_client()?;
        let endpoint = "/register";
        match client
            .post(&format!("http://{address}{endpoint}"))
            .json(&credentials)
            .send()
            .await
        {
            Ok(_) => {}
            Err(e) if e.status() == Some(StatusCode::CONFLICT) => return Err(Error::UsernameInUse),
            Err(e) => return Err(Self::handle_error(e, endpoint)),
        };

        Self::inner_login(address, credentials, client).await
    }

    async fn inner_login(
        address: &str,
        credentials: Credentials,
        client: HttpClient,
    ) -> Result<Self, Error> {
        let endpoint = "/auth/login";
        let login: LoginResult = match client
            .post(&format!("http://{address}{endpoint}"))
            .json(&credentials)
            .send()
            .await
        {
            Ok(response) if response.status() == StatusCode::UNAUTHORIZED => return Err(Error::LoginFailed),
            Ok(response) => response
                .json()
                .await
                .map_err(Error::DeserializingFailed)?,
            Err(e) => return Err(Self::handle_error(e, endpoint)),
        };

        Ok(Self {
            http_client: client,
            token: LoginToken(login.token),
            address: address.to_string(),
        })
    }

    pub async fn logout(&self) -> Result<(), Error> {
        let endpoint = "/auth/logout";
        match self
            .http_client
            .get(&format!("http://{}{endpoint}", self.address))
            .auth(self)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(Self::handle_error(e, endpoint)),
        }
    }

    pub async fn send_message(&self, message: &str) -> Result<(), Error> {
        let endpoint = "/message";
        match self
            .http_client
            .post(&format!("http://{}{endpoint}", self.address))
            .auth(self)
            .body(message.to_string())
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(Self::handle_error(e, endpoint)),
        }
    }

    pub async fn get_messages(&self, filter: MessageFilter) -> Result<Vec<Message>, Error> {
        let endpoint = "/messages";
        match self
            .http_client
            .post(&format!("http://{}{endpoint}", self.address))
            .auth(self)
            .json(&filter)
            .send()
            .await
        {
            Ok(response) => Ok(response
                .json()
                .await
                .map_err(Error::DeserializingFailed)?),
            Err(e) => Err(Self::handle_error(e, endpoint)),
        }
    }

    pub async fn get_users(&self, users: &Vec<i32>) -> Result<HashMap<i32, String>, Error> {
        let endpoint = "/user";
        match self
            .http_client
            .post(&format!("http://{}{endpoint}", self.address))
            .auth(self)
            .json(&users)
            .send()
            .await
        {
            Ok(response) => Ok(response
                .json()
                .await
                .map_err(Error::DeserializingFailed)?),
            Err(e) => Err(Self::handle_error(e, endpoint)),
        }
    }

    pub fn get_events(&self) -> Result<Receiver<Message>, Error> {
        let endpoint = "/events";

        let request = self
            .http_client
            .get(format!("http://{}{endpoint}", self.address))
            .auth(self);
        let mut event_source =
            EventSource::new(request).map_err(Error::EventSourceCreationFailed)?;

        let (tx, rx) = std::sync::mpsc::channel();

        tokio::spawn(async move { loop {
            while let Some(event) = event_source.next().await {
                match event {
                    Ok(Event::Message(message)) => {
                        if let Ok(message) = serde_json::from_str::<Message>(&message.data) {
                            if tx.send(message).is_err() {
                                return;
                            }
                        };
                    },
                    Err(_) => return,
                    _ => {}
                }
            }
        }});

        Ok(rx)
    }

    fn create_client() -> Result<HttpClient, Error> {
        HttpClient::builder()
            .build()
            .map_err(Error::ClientCreationFailed)
    }

    fn handle_error(error: reqwest::Error, endpoint: &str) -> Error {
        if error.is_connect() {
            return Error::ConnectionFailure(error);
        }
        if error.is_request() {
            return Error::InvalidRespone(error);
        }
        if error.is_status() {
            if let Some(code) = error.status() {
                if code == StatusCode::UNAUTHORIZED {
                    return Error::NotAuthorized;
                }
                return Error::UnexpectedStatusCode {
                    code,
                    endpoint: endpoint.to_string(),
                };
            }
        }

        Error::Generic(error)
    }
}

trait AuthResponse {
    fn auth(self, client: &Client) -> RequestBuilder;
}

impl AuthResponse for RequestBuilder {
    fn auth(self, client: &Client) -> RequestBuilder {
        self.bearer_auth(client.token.0.clone())
    }
}
