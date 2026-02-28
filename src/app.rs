use crossterm::event::KeyCode;
use std::env;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Main,
    TasteProfiles,
    DisplayProfiles,
    Build,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileMode {
    Browse,
    Adding(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildStep {
    PickTaste,
    PickDisplay,
    PickOutputDir,
}

#[derive(Debug, Clone)]
pub struct TasteProfile {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DisplayProfile {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainItem {
    TasteProfiles,
    DisplayProfiles,
    Build,
    Prune,
    Exit,
}

impl MainItem {
    pub const ALL: &'static [MainItem] = &[
        MainItem::TasteProfiles,
        MainItem::DisplayProfiles,
        MainItem::Build,
        MainItem::Prune,
        MainItem::Exit,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            MainItem::TasteProfiles => "Taste Profiles",
            MainItem::DisplayProfiles => "Display Profiles",
            MainItem::Build => "Build",
            MainItem::Prune => "Prune",
            MainItem::Exit => "Exit",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MainItem::TasteProfiles => {
                "Manage your art taste profiles (subjects, styles, periods)"
            }
            MainItem::DisplayProfiles => {
                "Manage your display profiles (resolution, aspect ratio, frame)"
            }
            MainItem::Build => {
                "Build a wallpaper gallery by picking a taste + display profile"
            }
            MainItem::Prune => "Remove old images based on retention limits (coming soon)",
            MainItem::Exit => "Exit artgg",
        }
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, MainItem::Prune)
    }
}

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,

    // Main menu
    pub main_selected: usize,

    // Taste profiles
    pub taste_profiles: Vec<TasteProfile>,
    pub taste_selected: usize,
    pub taste_mode: ProfileMode,

    // Display profiles
    pub display_profiles: Vec<DisplayProfile>,
    pub display_selected: usize,
    pub display_mode: ProfileMode,

    // Build wizard
    pub build_step: BuildStep,
    pub build_taste_idx: usize,
    pub build_display_idx: usize,
    pub build_output_dir: String,
}

impl App {
    pub fn new() -> Self {
        let default_output_dir = env::var("HOME")
            .map(|h| format!("{}/.local/share/artgg/gallery", h))
            .unwrap_or_else(|_| "~/.local/share/artgg/gallery".to_string());

        Self {
            screen: Screen::Main,
            should_quit: false,
            main_selected: 0,
            taste_profiles: Vec::new(),
            taste_selected: 0,
            taste_mode: ProfileMode::Browse,
            display_profiles: Vec::new(),
            display_selected: 0,
            display_mode: ProfileMode::Browse,
            build_step: BuildStep::PickTaste,
            build_taste_idx: 0,
            build_display_idx: 0,
            build_output_dir: default_output_dir,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        match self.screen {
            Screen::Main => self.handle_main(key),
            Screen::TasteProfiles => self.handle_taste(key),
            Screen::DisplayProfiles => self.handle_display(key),
            Screen::Build => self.handle_build(key),
        }
    }

    fn handle_main(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Char('k') => self.main_move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.main_move_down(),
            KeyCode::Enter => self.main_activate(),
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }

    fn main_move_up(&mut self) {
        let items = MainItem::ALL;
        let mut idx = if self.main_selected == 0 {
            items.len() - 1
        } else {
            self.main_selected - 1
        };
        while items[idx].is_disabled() {
            if idx == 0 {
                idx = items.len() - 1;
            } else {
                idx -= 1;
            }
        }
        self.main_selected = idx;
    }

    fn main_move_down(&mut self) {
        let items = MainItem::ALL;
        let mut idx = (self.main_selected + 1) % items.len();
        while items[idx].is_disabled() {
            idx = (idx + 1) % items.len();
        }
        self.main_selected = idx;
    }

    fn main_activate(&mut self) {
        match MainItem::ALL[self.main_selected] {
            MainItem::TasteProfiles => {
                self.screen = Screen::TasteProfiles;
                self.taste_mode = ProfileMode::Browse;
            }
            MainItem::DisplayProfiles => {
                self.screen = Screen::DisplayProfiles;
                self.display_mode = ProfileMode::Browse;
            }
            MainItem::Build => {
                self.build_step = BuildStep::PickTaste;
                self.build_taste_idx = 0;
                self.build_display_idx = 0;
                self.screen = Screen::Build;
            }
            MainItem::Prune => {}
            MainItem::Exit => self.should_quit = true,
        }
    }

    fn handle_taste(&mut self, key: KeyCode) {
        match self.taste_mode.clone() {
            ProfileMode::Browse => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if !self.taste_profiles.is_empty() && self.taste_selected > 0 {
                        self.taste_selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.taste_profiles.is_empty()
                        && self.taste_selected < self.taste_profiles.len() - 1
                    {
                        self.taste_selected += 1;
                    }
                }
                KeyCode::Char('a') => {
                    self.taste_mode = ProfileMode::Adding(String::new());
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    if !self.taste_profiles.is_empty() {
                        self.taste_profiles.remove(self.taste_selected);
                        if self.taste_selected > 0
                            && self.taste_selected >= self.taste_profiles.len()
                        {
                            self.taste_selected = self.taste_profiles.len() - 1;
                        }
                    }
                }
                KeyCode::Esc => {
                    self.screen = Screen::Main;
                }
                _ => {}
            },
            ProfileMode::Adding(mut buf) => match key {
                KeyCode::Char(c) => {
                    buf.push(c);
                    self.taste_mode = ProfileMode::Adding(buf);
                }
                KeyCode::Backspace => {
                    buf.pop();
                    self.taste_mode = ProfileMode::Adding(buf);
                }
                KeyCode::Enter => {
                    if !buf.is_empty() {
                        self.taste_profiles.push(TasteProfile { name: buf });
                        self.taste_selected = self.taste_profiles.len() - 1;
                        self.taste_mode = ProfileMode::Browse;
                    }
                }
                KeyCode::Esc => {
                    self.taste_mode = ProfileMode::Browse;
                }
                _ => {}
            },
        }
    }

    fn handle_display(&mut self, key: KeyCode) {
        match self.display_mode.clone() {
            ProfileMode::Browse => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if !self.display_profiles.is_empty() && self.display_selected > 0 {
                        self.display_selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.display_profiles.is_empty()
                        && self.display_selected < self.display_profiles.len() - 1
                    {
                        self.display_selected += 1;
                    }
                }
                KeyCode::Char('a') => {
                    self.display_mode = ProfileMode::Adding(String::new());
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    if !self.display_profiles.is_empty() {
                        self.display_profiles.remove(self.display_selected);
                        if self.display_selected > 0
                            && self.display_selected >= self.display_profiles.len()
                        {
                            self.display_selected = self.display_profiles.len() - 1;
                        }
                    }
                }
                KeyCode::Esc => {
                    self.screen = Screen::Main;
                }
                _ => {}
            },
            ProfileMode::Adding(mut buf) => match key {
                KeyCode::Char(c) => {
                    buf.push(c);
                    self.display_mode = ProfileMode::Adding(buf);
                }
                KeyCode::Backspace => {
                    buf.pop();
                    self.display_mode = ProfileMode::Adding(buf);
                }
                KeyCode::Enter => {
                    if !buf.is_empty() {
                        self.display_profiles.push(DisplayProfile { name: buf });
                        self.display_selected = self.display_profiles.len() - 1;
                        self.display_mode = ProfileMode::Browse;
                    }
                }
                KeyCode::Esc => {
                    self.display_mode = ProfileMode::Browse;
                }
                _ => {}
            },
        }
    }

    fn handle_build(&mut self, key: KeyCode) {
        match self.build_step {
            BuildStep::PickTaste => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.build_taste_idx > 0 {
                        self.build_taste_idx -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.taste_profiles.is_empty()
                        && self.build_taste_idx < self.taste_profiles.len() - 1
                    {
                        self.build_taste_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    if !self.taste_profiles.is_empty() {
                        self.build_step = BuildStep::PickDisplay;
                    }
                }
                KeyCode::Esc => {
                    self.screen = Screen::Main;
                }
                _ => {}
            },
            BuildStep::PickDisplay => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.build_display_idx > 0 {
                        self.build_display_idx -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.display_profiles.is_empty()
                        && self.build_display_idx < self.display_profiles.len() - 1
                    {
                        self.build_display_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    if !self.display_profiles.is_empty() {
                        self.build_step = BuildStep::PickOutputDir;
                    }
                }
                KeyCode::Esc => {
                    self.build_step = BuildStep::PickTaste;
                }
                _ => {}
            },
            BuildStep::PickOutputDir => match key {
                KeyCode::Char(c) => {
                    self.build_output_dir.push(c);
                }
                KeyCode::Backspace => {
                    self.build_output_dir.pop();
                }
                KeyCode::Enter => {
                    self.screen = Screen::Main;
                }
                KeyCode::Esc => {
                    self.build_step = BuildStep::PickDisplay;
                }
                _ => {}
            },
        }
    }
}
