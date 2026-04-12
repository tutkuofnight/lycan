pub mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::pwa::{config, desktop, favicon};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Form(FormKind),
    ConfirmDelete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormKind {
    Add,
    Edit(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormField {
    Name,
    Url,
}

pub struct App {
    pub mode: Mode,
    pub apps: Vec<config::AppConfig>,
    pub selected: usize,
    pub form_field: FormField,
    pub name_input: String,
    pub url_input: String,
    pub search_query: String,
    pub filtered_indices: Vec<usize>,
    pub status_message: Option<(String, bool)>, // (message, is_error)
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let apps = config::list_apps()?;
        let filtered_indices: Vec<usize> = (0..apps.len()).collect();
        Ok(Self {
            mode: Mode::Normal,
            apps,
            selected: 0,
            form_field: FormField::Name,
            name_input: String::new(),
            url_input: String::new(),
            search_query: String::new(),
            filtered_indices,
            status_message: None,
            should_quit: false,
        })
    }

    pub fn refresh_apps(&mut self) -> Result<()> {
        self.apps = config::list_apps()?;
        self.apply_filter();
        if self.selected >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
        }
        Ok(())
    }

    pub fn selected_app(&self) -> Option<&config::AppConfig> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.apps.get(i))
    }

    fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            self.filtered_indices = (0..self.apps.len()).collect();
        } else {
            self.filtered_indices = self
                .apps
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    a.name.to_lowercase().contains(&query)
                        || a.url.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

    fn enter_add_mode(&mut self) {
        self.name_input.clear();
        self.url_input.clear();
        self.form_field = FormField::Name;
        self.mode = Mode::Form(FormKind::Add);
        self.status_message = None;
    }

    fn enter_edit_mode(&mut self) {
        if let Some(&idx) = self.filtered_indices.get(self.selected) {
            if let Some(app) = self.apps.get(idx) {
                self.name_input = app.name.clone();
                self.url_input = app.url.clone();
                self.form_field = FormField::Name;
                self.mode = Mode::Form(FormKind::Edit(idx));
                self.status_message = None;
            }
        }
    }

    fn save_form(&mut self) -> Result<()> {
        let name = self.name_input.trim().to_string();
        let mut url = self.url_input.trim().to_string();

        if name.is_empty() || url.is_empty() {
            self.status_message = Some(("Name and URL are required.".into(), true));
            return Ok(());
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            url = format!("https://{}", url);
        }

        if url::Url::parse(&url).is_err() {
            self.status_message = Some(("Invalid URL.".into(), true));
            return Ok(());
        }

        match &self.mode {
            Mode::Form(FormKind::Add) => {
                let app_config = config::AppConfig::new(&name, &url);

                if config::app_dir(&app_config.id)?
                    .join("config.json")
                    .exists()
                {
                    self.status_message =
                        Some((format!("'{}' already exists.", app_config.id), true));
                    return Ok(());
                }

                config::save(&app_config)?;

                let icon_path = config::icon_path(&app_config.id)?;
                let _ = favicon::fetch_and_save(&url, &icon_path);

                desktop::create(&app_config)?;

                self.status_message =
                    Some((format!("Created '{}'.", app_config.name), false));
            }
            Mode::Form(FormKind::Edit(idx)) => {
                if let Some(app) = self.apps.get(*idx) {
                    let id = app.id.clone();
                    config::update(&id, &name, &url)?;
                    self.status_message = Some((format!("Updated '{}'.", name), false));
                }
            }
            _ => {}
        }

        self.mode = Mode::Normal;
        self.refresh_apps()?;
        Ok(())
    }

    fn delete_selected(&mut self) -> Result<()> {
        if let Some(app) = self.selected_app() {
            let name = app.name.clone();
            let id = app.id.clone();
            desktop::remove(&id)?;
            config::delete(&id)?;
            self.refresh_apps()?;
            self.status_message = Some((format!("Deleted '{}'.", name), false));
            self.mode = Mode::Normal;
        }
        Ok(())
    }

    fn open_selected(&self) -> Result<()> {
        if let Some(app) = self.selected_app() {
            let exe = std::env::current_exe().unwrap_or_else(|_| "lycan".into());
            std::process::Command::new(exe)
                .arg("open")
                .arg(&app.id)
                .spawn()?;
        }
        Ok(())
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected < self.filtered_indices.len() - 1 {
            self.selected += 1;
        }
    }
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match &app.mode {
                Mode::Normal => handle_normal_keys(&mut app, key.code)?,
                Mode::Search => handle_search_keys(&mut app, key.code)?,
                Mode::Form(_) => handle_form_keys(&mut app, key.code, key.modifiers)?,
                Mode::ConfirmDelete => handle_confirm_keys(&mut app, key.code)?,
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn handle_normal_keys(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('a') => app.enter_add_mode(),
        KeyCode::Char('e') => app.enter_edit_mode(),
        KeyCode::Char('o') | KeyCode::Enter => app.open_selected()?,
        KeyCode::Char('d') => {
            if app.selected_app().is_some() {
                app.mode = Mode::ConfirmDelete;
                app.status_message = None;
            }
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query.clear();
            app.status_message = None;
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        _ => {}
    }
    Ok(())
}

fn handle_search_keys(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Esc => {
            app.search_query.clear();
            app.apply_filter();
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_filter();
        }
        KeyCode::Up | KeyCode::Down => {
            if key == KeyCode::Up {
                app.move_up();
            } else {
                app.move_down();
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_form_keys(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
    if modifiers.contains(KeyModifiers::CONTROL) && key == KeyCode::Char('s') {
        app.save_form()?;
        return Ok(());
    }

    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.status_message = None;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            app.form_field = match app.form_field {
                FormField::Name => FormField::Url,
                FormField::Url => FormField::Name,
            };
        }
        KeyCode::Enter => {
            if app.form_field == FormField::Name {
                app.form_field = FormField::Url;
            } else {
                app.save_form()?;
            }
        }
        KeyCode::Backspace => {
            let input = match app.form_field {
                FormField::Name => &mut app.name_input,
                FormField::Url => &mut app.url_input,
            };
            input.pop();
        }
        KeyCode::Char(c) => {
            let input = match app.form_field {
                FormField::Name => &mut app.name_input,
                FormField::Url => &mut app.url_input,
            };
            input.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_confirm_keys(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.delete_selected()?,
        _ => {
            app.mode = Mode::Normal;
            app.status_message = None;
        }
    }
    Ok(())
}
