use std::{
    cmp::Ordering,
    collections::HashMap,
    io::{self},
    time::Duration,
};

use chat_app::{models::Message, MessageFilter};
use chrono::Local;
use client::Client;
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
use tokio::sync::mpsc::{channel, error::TryRecvError, Receiver, Sender};
use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Paragraph, Tabs},
    Frame, Terminal,
};

mod client;
mod collections;
mod screens;

#[tokio::main]
async fn main() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let (mut app, mut shutdown_receiver) = App::new();
    let app_task = tokio::spawn(async move {
        let result = run_app(&mut terminal, &mut app).await;

        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    });
    let app_result = app_task.await?;
    shutdown_receiver.recv().await;
    app_result
}

/// Main loop for running the app.
async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B: std::io::Write,
{
    loop {
        for session in app.chat.logins.values_mut() {
            session.update().await?;
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
                    } => {
                        app.shutdown.cancel();
                        break;
                    }
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
                            screen.handle_input(&mut app.chat, &event).await;
                        }
                    }
                }
            };
        }
    }

    // logout all clients
    for (username, session) in &app.chat.logins {
        let result = session.client.logout().await;
        if let Err(e) = result {
            println!("Error whilst logging out as {username}: {e}");
        }
    }

    Ok(())
}

/// Update the ui.
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

/// Creates a ``Paragraph`` holding the help text shown at the bottom.
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

/// Converts instances of ``TabTitle`` to a collection of ``Spans``.
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

/// Holds the name and state of the title for the tabs.
enum TabTitle {
    Active(String),
    Inactive(String),
}

/// Holds the current state of the the app and ui.
struct App {
    chat: ChatData,
    screens: ActiveVec<Window>,
    shutdown: ShutdownHandler,
}

/// Holds the data relating to the current state of the application
struct ChatData {
    logins: HashMap<String, SessionData>,
}

/// Holds the data for a users session.
struct SessionData {
    client: Client,
    events: Receiver<Message>,
    messages: Vec<Message>,
    known_usernames: HashMap<i32, String>,
}

///
struct ShutdownHandler {
    token: CancellationToken,
    sender: Sender<()>,
}

impl ShutdownHandler {
    fn new() -> (Self, Receiver<()>) {
        let (sender, receiver) = channel(2);
        let token = CancellationToken::new();
        (Self { token, sender }, receiver)
    }

    pub fn child(&self) -> Self {
        Self {
            token: self.token.child_token(),
            sender: self.sender.clone(),
        }
    }

    pub fn cancel(&self) {
        self.token.cancel()
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn cancelled(&self) -> WaitForCancellationFuture<'_> {
        self.token.cancelled()
    }
}

impl App {
    /// Create a new instance of ``App``.
    fn new() -> (Self, Receiver<()>) {
        let mut screen: ActiveVec<Window> = ActiveVec::new();
        screen.push(Window::new());

        let chat = ChatData {
            logins: HashMap::new(),
        };

        let (shutdown, receiver) = ShutdownHandler::new();

        (
            App {
                chat,
                screens: screen,
                shutdown,
            },
            receiver,
        )
    }

    /// Get the ``TabTitle``s to show.
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

impl SessionData {
    /// Creates a new instance of ``SessionData`` and populates it with chat messages
    async fn new(client: Client) -> Result<Self> {
        let events = client.get_events()?;
        let now = Local::now();
        let mut messages = client.get_messages(MessageFilter::Before(now)).await?;
        messages.sort_by(Self::sort_messages);
        let known_usernames: HashMap<i32, String> = HashMap::new();
        let mut session = Self {
            client,
            events,
            messages,
            known_usernames,
        };

        session.update_names().await?;

        Ok(session)
    }

    /// Updates the sessions states and adds new messages if available.
    async fn update(&mut self) -> Result<()> {
        loop {
            let message = match self.events.try_recv() {
                Ok(message) => message,
                Err(e) => match e {
                    TryRecvError::Empty => break,
                    TryRecvError::Disconnected => return Err(eyre::eyre!("Server disconnected!")),
                },
            };

            self.messages.push(message);
        }

        self.update_names().await?;

        Ok(())
    }

    async fn update_names(&mut self) -> Result<()> {
        let mut missing_ids: Vec<i32> = self
            .messages
            .iter()
            .filter_map(|m| {
                if self.known_usernames.contains_key(&m.userid) {
                    None
                } else {
                    Some(m.userid)
                }
            })
            .collect();

        if !missing_ids.is_empty() {
            missing_ids.dedup();
            let users = self.client.get_users(&missing_ids).await?;
            self.known_usernames.extend(users);
        }

        Ok(())
    }

    fn sort_messages(a: &Message, b: &Message) -> Ordering {
        a.date.cmp(&b.date)
    }
}
