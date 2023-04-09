use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use tui::{
    buffer::Buffer,
    layout::{Alignment::Center, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::{
    client::{AuthDetails, Client},
    ChatData, SessionData,
};

/// Used to hold the current window state.
#[derive(Clone)]
pub struct Window {
    state: MenuState,
}

/// Keeps track of what ste the ``Window`` currently is in.
#[derive(Clone)]
enum MenuState {
    Chat(ChatWindow),
    Login(LoginWindow),
}

/// Holds the current state of the chat window.
#[derive(Clone)]
struct ChatWindow {
    title: String,
    message_list: Vec<String>,
    message_composer: String,
    status_message: Option<String>,
}

/// Holds the current state of the login window.
#[derive(Clone)]
struct LoginWindow {
    address: FormElement,
    username: FormElement,
    password: FormElement,
    intent: Intent,
    focus: LoginWindowFocus,
    status_message: Option<String>,
}

/// What does the user wanna do when they hit enter?
#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    Login,
    Register,
}

/// Keeps track of what element in the ``LoginWindow`` is currently in focus.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LoginWindowFocus {
    Address,
    Username,
    Pasword,
    Intent,
}

/// Represents a form element in a ui screen.
#[derive(Clone)]
struct FormElement {
    title: String,
    content: String,
    visibilty: Visibilty,
}

/// Indicates whether the contents of a ``FormElement`` should be shown or replaced by asterisks.
#[derive(Clone)]
enum Visibilty {
    Visible,
    Hidden,
}

impl FormElement {
    /// Creates a new ``FormElement``.
    fn new(title: &str, visibilty: Visibilty) -> Self {
        Self {
            content: String::new(),
            title: title.into(),
            visibilty,
        }
    }
}

impl Window {
    /// Creates a new ``Window`` instance.
    pub fn new() -> Self {
        Self {
            state: MenuState::Login(LoginWindow {
                address: FormElement::new("Server Address", Visibilty::Visible),
                username: FormElement::new("Username", Visibilty::Visible),
                password: FormElement::new("Password", Visibilty::Hidden),
                intent: Intent::Login,
                focus: LoginWindowFocus::Address,
                status_message: None,
            }),
        }
    }

    /// Get the current title for the window.
    pub fn title(&self) -> String {
        match &self.state {
            MenuState::Chat(window) => window.title.clone(),
            MenuState::Login(_) => "Log in".into(),
        }
    }

    /// Handles the input for the window and apply changes to it and the ``ChatData`` as necessary.
    pub(crate) async fn handle_input(&mut self, data: &mut ChatData, event: &Event) {
        // If the event is from a a key release, we ignore it
        if let Event::Key(KeyEvent { code: _, modifiers: _, kind: KeyEventKind::Release, state: _ }) = event {
            return;
        }
        match &mut self.state {
            MenuState::Chat(chat) => {
                handle_chat_window_input(chat, event, data).await;
            }
            MenuState::Login(form) => {
                // Clone the state and passing it in like that is a bit awkward.
                // But so far the best solution I could come up with as I otherwise
                // need to pass in two mutable instances of self which causes a compiler error.
                // There must be a better way to go about this, but I am not seeing it yet.
                let mut form = form.clone();
                self.handle_login_window_input(&mut form, event, data).await;
                if let MenuState::Login(_) = self.state {
                    self.state = MenuState::Login(form);
                }
            }
        }
    }

    async fn handle_login_window_input(
        &mut self,
        form: &mut LoginWindow,
        event: &Event,
        data: &mut ChatData,
    ) {
        if form.status_message.is_some() {
            //form.status_message = None;
        }
        if let Event::Key(KeyEvent {
            code,
            modifiers: _,
            kind: _,
            state: _,
        }) = event
        {
            match code {
                KeyCode::Up => {
                    form.focus = match form.focus {
                        LoginWindowFocus::Address | LoginWindowFocus::Username => {
                            LoginWindowFocus::Address
                        }
                        LoginWindowFocus::Pasword => LoginWindowFocus::Username,
                        LoginWindowFocus::Intent => LoginWindowFocus::Pasword,
                    };
                }
                KeyCode::Down => {
                    form.focus = match form.focus {
                        LoginWindowFocus::Address => LoginWindowFocus::Username,
                        LoginWindowFocus::Username => LoginWindowFocus::Pasword,
                        LoginWindowFocus::Pasword | LoginWindowFocus::Intent => {
                            LoginWindowFocus::Intent
                        }
                    }
                }
                KeyCode::Left if form.focus == LoginWindowFocus::Intent => {
                    form.intent = Intent::Login;
                }
                KeyCode::Right if form.focus == LoginWindowFocus::Intent => {
                    form.intent = Intent::Register;
                }
                KeyCode::Enter => {
                    self.submit_form(form, data).await;
                }
                KeyCode::Char(c) => match form.focus {
                    LoginWindowFocus::Address => form.address.content.push(*c),
                    LoginWindowFocus::Username => form.username.content.push(*c),
                    LoginWindowFocus::Pasword => form.password.content.push(*c),
                    LoginWindowFocus::Intent => {}
                },
                KeyCode::Backspace => {
                    match form.focus {
                        LoginWindowFocus::Address => {
                            form.address.content.pop();
                        }
                        LoginWindowFocus::Username => {
                            form.username.content.pop();
                        }
                        LoginWindowFocus::Pasword => {
                            form.password.content.pop();
                        }
                        LoginWindowFocus::Intent => {}
                    };
                }
                _ => {}
            }
        }
    }

    async fn submit_form(&mut self, form: &mut LoginWindow, data: &mut ChatData) {
        let auth_details = AuthDetails::new(
            &form.address.content,
            &form.username.content,
            &form.password.content,
        );
        let result = match form.intent {
            Intent::Login => Client::login(auth_details).await,
            Intent::Register => Client::register(auth_details).await,
        };
        match result {
            Ok(client) => {
                let username = &form.username.content;
                match SessionData::new(client).await {
                    Ok(session) => {
                        data.logins.insert(username.clone(), session);
                        self.state = MenuState::Chat(ChatWindow {
                            title: username.clone(),
                            message_list: Vec::new(),
                            message_composer: String::new(),
                            status_message: None,
                        });
                    }
                    Err(e) => {
                        form.status_message = Some(format!("Could not create session: {e}"));
                    }
                }
            }
            Err(e) => {
                form.status_message = Some(format!("Login failed. ({e})"));
            }
        }
    }

    /// Updates the ui state with the ``SessionData``.
    pub(crate) fn update(&mut self, data: &SessionData) {
        match &mut self.state {
            MenuState::Chat(chat) => {
                let mut messages: Vec<String> = Vec::new();

                for message in &data.messages {
                    let name = match data.known_usernames.get(&message.userid) {
                        Some(name) => name.clone(),
                        None => message.userid.to_string(),
                    };
                    let text = &message.messagetext;
                    messages.push(format!("{name}: {text}"));
                }

                chat.message_list = messages;
            }
            MenuState::Login(_) => {}
        }
    }
}

async fn handle_chat_window_input(chat: &mut ChatWindow, event: &Event, data: &mut ChatData) {
    if chat.status_message.is_some() {
        chat.status_message = None;
    }
    if let Event::Key(KeyEvent {
        code,
        modifiers: _,
        kind: _,
        state: _,
    }) = event
    {
        match code {
            KeyCode::Enter => {
                if let Some(session_data) = data.logins.get(&chat.title) {
                    let result = session_data
                        .client
                        .send_message(&chat.message_composer)
                        .await;
                    let message = if let Err(e) = result {
                        format!("Could not send message: {e}")
                    } else {
                        chat.message_composer.clear();
                        "Message sent.".into()
                    };

                    chat.status_message = Some(message);
                }
            }
            KeyCode::Char(c) => {
                chat.message_composer.push(*c);
            }
            KeyCode::Backspace => {
                chat.message_composer.pop();
            }
            _ => {}
        }
    }
}

impl Widget for Window {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::TOP);
        let inner = block.inner(area);
        block.render(area, buf);

        match self.state {
            // Rendering logic for the chat screen
            MenuState::Chat(chat) => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(10),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ])
                    .split(inner);

                let message_count = (layout[0].height - 2) as usize;
                let items: Vec<ListItem> = chat
                    .message_list
                    .iter()
                    .rev() // First reverse the order
                    .take(message_count) // So we can take out the last n elements
                    .rev() // Then reverse it again so it's in the correct order again
                    .map(|m| ListItem::new(Text::from(m.clone())))
                    .collect();
                tui::widgets::Widget::render(
                    List::new(items).block(Block::default().borders(Borders::ALL)),
                    layout[0],
                    buf,
                );

                Paragraph::new(Span::styled(chat.message_composer, Style::default()))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Yellow)),
                    )
                    .render(layout[1], buf);

                if let Some(message) = chat.status_message {
                    Paragraph::new(Span::styled(message, Style::default())).render(layout[2], buf);
                }
            }
            // Rendering logic for the login screen
            MenuState::Login(login) => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ])
                    .split(inner);

                form_element_ui(&login.address, login.focus == LoginWindowFocus::Address)
                    .render(layout[0], buf);
                form_element_ui(&login.username, login.focus == LoginWindowFocus::Username)
                    .render(layout[1], buf);
                form_element_ui(&login.password, login.focus == LoginWindowFocus::Pasword)
                    .render(layout[2], buf);

                let style = if login.focus == LoginWindowFocus::Intent {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                let (login_style, register_style) = match login.intent {
                    Intent::Login => (
                        style.add_modifier(Modifier::UNDERLINED),
                        style.fg(Color::DarkGray),
                    ),
                    Intent::Register => (
                        style.fg(Color::DarkGray),
                        style.add_modifier(Modifier::UNDERLINED),
                    ),
                };

                Paragraph::new(Spans::from(vec![
                    Span::styled("Login as a existing user", login_style),
                    Span::styled(" | ", Style::default()),
                    Span::styled("Register as a new user", register_style),
                ]))
                .render(layout[3], buf);

                if let Some(message) = login.status_message {
                    Paragraph::new(Span::styled(message, Style::default()))
                        .alignment(Center)
                        .render(layout[4], buf);
                }

                Paragraph::new(Span::styled("Press Enter to submit.", Style::default()))
                    .alignment(Center)
                    .render(layout[5], buf);
            }
        }
    }
}

/// Creates a ``Paragraph`` widget for the given ``FormElement``.
fn form_element_ui<'a>(element: &FormElement, active: bool) -> Paragraph<'a> {
    let active_style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let content = match element.visibilty {
        Visibilty::Visible => element.content.clone(),
        Visibilty::Hidden => "*".repeat(element.content.len()),
    };

    Paragraph::new(Span::styled(content, Style::default())).block(
        Block::default()
            .title(Span::styled(element.title.clone(), active_style))
            .borders(Borders::ALL)
            .border_style(active_style),
    )
}
