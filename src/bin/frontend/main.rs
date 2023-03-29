use std::{
    collections::HashMap,
    io::{self},
    time::Duration,
};

use chat_app::{models::Message, ChatApp, LoginToken, MessageFilter};
use chrono::{DateTime, Local};
use collections::ActiveVec;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use eyre::Result;
use screens::Window;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Paragraph, Tabs},
    Frame, Terminal,
};

mod collections;
mod screens;

fn main() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new()?;
    run_app(&mut terminal, &mut app)?;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        for session in app.chat.logins.values_mut() {
            session.update(&mut app.chat.chat_app)?;
        }

        if let Some(screen) = app.screens.get_active_mut() {
            if let Some(session) = app.chat.logins.get(&screen.title()) {
                screen.update(session);
            }
        }

        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                match key {
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: _,
                        state: _,
                    } => return Ok(()),
                    KeyEvent {
                        code: KeyCode::Char('n'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: _,
                        state: _,
                    } => {
                        app.screens.push(Window::new());
                        app.screens.next();
                    }
                    KeyEvent {
                        code: KeyCode::Tab,
                        modifiers: _,
                        kind: _,
                        state: _,
                    } => app.screens.next(),
                    KeyEvent {
                        code: KeyCode::BackTab,
                        modifiers: _,
                        kind: _,
                        state: _,
                    } => app.screens.prev(),
                    _ => {
                        if let Some(screen) = app.screens.get_active_mut() {
                            screen.handle_input(&mut app.chat, &event);
                        }
                    }
                }
            };
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Min(9),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(size);

    let titles = &app.tab_titles();
    let tabs = Tabs::new(tab_titles_to_spans(titles));
    f.render_widget(tabs, chunks[0]);

    if let Some(window) = app.screens.get_active() {
        f.render_widget(window.clone(), chunks[1]);
    }

    f.render_widget(help_text(), chunks[2]);
}

fn help_text<'a>() -> Paragraph<'a> {
    let normal = Style::default();
    let highlight = Style::default().fg(Color::Green);

    Paragraph::new(Spans::from(vec![
        Span::styled("Press ", normal),
        Span::styled("Ctrl+q", highlight),
        Span::styled(" to exit. Press ", normal),
        Span::styled("Tab", highlight),
        Span::styled(" to switch between windows. Press ", normal),
        Span::styled("Ctrl+n", highlight),
        Span::styled(" to open a new window.", normal),
    ]))
}

fn tab_titles_to_spans(titles: &[TabTitle]) -> Vec<Spans> {
    titles
        .iter()
        .map(|title| match title {
            TabTitle::Active(text) => {
                Spans::from(Span::styled(text, Style::default().fg(Color::Yellow)))
            }
            TabTitle::Inactive(text) => Spans::from(Span::styled(text, Style::default())),
        })
        .collect()
}

enum TabTitle {
    Active(String),
    Inactive(String),
}

struct App {
    chat: ChatData,
    screens: ActiveVec<Window>,
}

struct ChatData {
    chat_app: ChatApp,
    logins: HashMap<String, SessionData>,
}

struct SessionData {
    token: LoginToken,
    last_update: DateTime<Local>,
    messages: Vec<Message>,
    known_usernames: HashMap<i32, String>,
}

impl SessionData {
    fn new(app: &mut ChatApp, token: LoginToken) -> Result<Self> {
        let now = Local::now();
        let messages = app.get_messages(&token, &MessageFilter::Before(now))?;
        let mut known_usernames: HashMap<i32, String> = HashMap::new();
        for msg in &messages {
            if !known_usernames.contains_key(&msg.userid) {
                if let Ok(user) = app.get_user_by_id(msg.userid) {
                    known_usernames.insert(user.id, user.username);
                }
            }
        }

        Ok(Self {
            token,
            last_update: now,
            messages,
            known_usernames,
        })
    }

    fn update(&mut self, app: &mut ChatApp) -> Result<()> {
        let now = Local::now();
        let mut messages =
            app.get_messages(&self.token, &MessageFilter::After(self.last_update))?;
        if !messages.is_empty() {
            for msg in &messages {
                if !self.known_usernames.contains_key(&msg.userid) {
                    if let Ok(user) = app.get_user_by_id(msg.userid) {
                        self.known_usernames.insert(user.id, user.username);
                    }
                }
            }
            self.messages.append(&mut messages);
            self.last_update = now;
        }

        Ok(())
    }
}

impl App {
    fn new() -> Result<Self> {
        let mut screen: ActiveVec<Window> = ActiveVec::new();
        screen.push(Window::new());

        let chat = ChatData {
            chat_app: ChatApp::new()?,
            logins: HashMap::new(),
        };

        Ok(App {
            chat,
            screens: screen,
        })
    }

    fn tab_titles(&self) -> Vec<TabTitle> {
        if let Some(active_index) = self.screens.get_active_index() {
            self.screens
                .iter()
                .enumerate()
                .map(|(index, screen)| {
                    if index == active_index {
                        TabTitle::Active(screen.title())
                    } else {
                        TabTitle::Inactive(screen.title())
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }
}
