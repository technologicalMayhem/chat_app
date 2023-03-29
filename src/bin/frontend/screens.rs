use crossterm::event::{Event, KeyCode, KeyEvent};
use tui::{
    buffer::Buffer,
    layout::{Alignment::Center, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::{ChatData, SessionData};

#[derive(Clone)]
pub struct Window {
    state: MenuState,
}

#[derive(Clone)]
enum MenuState {
    Chat(ChatWindow),
    Login(LoginWindow),
}

#[derive(Clone)]
struct ChatWindow {
    title: String,
    message_list: Vec<String>,
    message_composer: String,
    status_message: Option<String>,
}

#[derive(Clone)]
struct LoginWindow {
    username: FormElement,
    password: FormElement,
    focus: LoginWindowFocus,
    status_message: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoginWindowFocus {
    Username,
    Pasword,
}

#[derive(Clone)]
struct FormElement {
    title: String,
    content: String,
    visibilty: Visibilty,
}

#[derive(Clone)]
enum Visibilty {
    Visible,
    Hidden,
}

impl FormElement {
    fn new(title: &str, visibilty: Visibilty) -> Self {
        Self {
            content: String::new(),
            title: title.into(),
            visibilty,
        }
    }
}

impl Window {
    pub fn new() -> Self {
        Self {
            state: MenuState::Login(LoginWindow {
                username: FormElement::new("Username", Visibilty::Visible),
                password: FormElement::new("Password", Visibilty::Hidden),
                focus: LoginWindowFocus::Username,
                status_message: None,
            }),
        }
    }
}

impl Window {
    pub fn title(&self) -> String {
        match &self.state {
            MenuState::Chat(window) => window.title.clone(),
            MenuState::Login(_) => "Log in".into(),
        }
    }

    pub(crate) fn handle_input(&mut self, data: &mut ChatData, event: &Event) {
        match &mut self.state {
            MenuState::Chat(chat) => {
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
                                let result = data
                                    .chat_app
                                    .send_message(&session_data.token, &chat.message_composer);

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
            MenuState::Login(form) => {
                if form.status_message.is_some() {
                    form.status_message = None;
                }
                if let Event::Key(KeyEvent {
                    code,
                    modifiers: _,
                    kind: _,
                    state: _,
                }) = event
                {
                    match code {
                        KeyCode::Up => form.focus = LoginWindowFocus::Username,
                        KeyCode::Down => form.focus = LoginWindowFocus::Pasword,
                        KeyCode::Enter => {
                            if let Ok(token) = data
                                .chat_app
                                .login(&form.username.content, &form.password.content)
                            {
                                let username = &form.username.content;
                                match SessionData::new(&mut data.chat_app, token) {
                                    Ok(session) => {
                                        data.logins.insert(username.clone(), session);
                                        self.state = MenuState::Chat(ChatWindow {
                                            title: username.clone(),
                                            message_list: Vec::new(),
                                            message_composer: String::new(),
                                            status_message: None,
                                        });
                                    }
                                    Err(e) => form.status_message = Some(format!("Could not create session: {e}")),
                                }
                            } else {
                                form.status_message = Some("Login failed".into());
                            }
                        }
                        KeyCode::Char(c) => match form.focus {
                            LoginWindowFocus::Username => form.username.content.push(*c),
                            LoginWindowFocus::Pasword => form.password.content.push(*c),
                        },
                        KeyCode::Backspace => {
                            match form.focus {
                                LoginWindowFocus::Username => form.username.content.pop(),
                                LoginWindowFocus::Pasword => form.password.content.pop(),
                            };
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub(crate) fn update(&mut self, data: &SessionData) {
        match &mut self.state {
            MenuState::Chat(chat) => {
                let mut messages: Vec<String> = Vec::new();

                for message in &data.messages {
                    messages.push(format!("{}: {}", message.userid, message.messagetext));
                }

                chat.message_list = messages;
            },
            MenuState::Login(_) => {},
        }
    }
}

impl Widget for Window {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::TOP);
        let inner = block.inner(area);
        block.render(area, buf);

        match self.state {
            MenuState::Chat(chat) => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(10),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ])
                    .split(inner);

                let items: Vec<ListItem> = chat
                    .message_list
                    .iter()
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
            MenuState::Login(login) => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ])
                    .split(inner);

                form_element_ui(&login.username, login.focus == LoginWindowFocus::Username)
                    .render(layout[0], buf);
                form_element_ui(&login.password, login.focus == LoginWindowFocus::Pasword)
                    .render(layout[1], buf);

                if let Some(message) = login.status_message {
                    Paragraph::new(Span::styled(message, Style::default()))
                        .alignment(Center)
                        .render(layout[2], buf);
                }

                Paragraph::new(Span::styled("Press Enter to submit.", Style::default()))
                    .alignment(Center)
                    .render(layout[3], buf);
            }
        }
    }
}

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
