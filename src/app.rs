use anyhow::Result;
use crossterm::event::KeyCode;
use rusqlite::Connection;
use std::env;

use crate::db;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Main,
    TasteProfiles,
    DisplayProfiles,
    Build,
}

/// Mode for the Taste Profiles screen.
#[derive(Debug, Clone, PartialEq)]
pub enum TasteScreenMode {
    Browse,
    Detail,
    EditingDate(String),         // typing year for existing profile
    SelectingKeywords,           // keyword picker for existing profile
    CreatingProfile,             // creation form (navigating fields 0-4)
    CreatingEditDate(String),    // typing year inside creation form
    CreatingSelectKeywords,      // keyword picker inside creation form
    CreatingName(String),        // typing name — last step of creation
}

/// Mode for the Display Profiles screen.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayScreenMode {
    Browse,
    Detail,
    EditingText(String),         // typing text for existing profile (color or ratio)
    CreatingProfile,             // creation form (navigating fields 0-4)
    CreatingEditText(String),    // typing text inside creation form
    CreatingName(String),        // typing name — last step of creation
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildStep {
    PickTaste,
    PickDisplay,
    PickOutputDir,
}

#[derive(Debug, Clone)]
pub struct TasteProfile {
    pub id: i64,
    pub name: String,
    pub date_start: Option<i64>,
    pub date_end: Option<i64>,
    pub is_public_domain: bool,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DisplayProfile {
    pub id: i64,
    pub name: String,
    pub wallpaper_color: String,
    pub frame_style: String,
    pub orientation: String,   // "horizontal" or "vertical"
    pub aspect_ratio: String,
}

/// Draft state held while creating a new taste profile.
#[derive(Debug, Clone)]
pub struct TasteProfileDraft {
    pub date_start: Option<i64>,
    pub date_end: Option<i64>,
    pub is_public_domain: bool,
    pub keywords: Vec<String>,
    pub name: String,
    pub current_field: usize, // 0=date_start 1=date_end 2=pd 3=keywords 4=name
}

impl Default for TasteProfileDraft {
    fn default() -> Self {
        Self {
            date_start: None,
            date_end: None,
            is_public_domain: true, // default Yes
            keywords: vec![],
            name: String::new(),
            current_field: 0,
        }
    }
}

/// Draft state held while creating a new display profile.
#[derive(Debug, Clone)]
pub struct DisplayProfileDraft {
    pub wallpaper_color: String,
    pub frame_style: String,
    pub orientation: String,
    pub aspect_ratio: String,
    pub name: String,
    pub current_field: usize, // 0=color 1=frame 2=orientation 3=ratio 4=name
}

impl Default for DisplayProfileDraft {
    fn default() -> Self {
        Self {
            wallpaper_color: "#FFFFFF".to_string(),
            frame_style: String::new(),
            orientation: "horizontal".to_string(),
            aspect_ratio: "16:9".to_string(),
            name: String::new(),
            current_field: 0,
        }
    }
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
    pub taste_mode: TasteScreenMode,
    pub taste_detail_field: usize, // 0=date_start 1=date_end 2=pd 3=keywords 4=artists
    pub available_keywords: Vec<(i64, String)>,
    pub keyword_cursor: usize,
    pub new_taste_draft: TasteProfileDraft,

    // Display profiles
    pub display_profiles: Vec<DisplayProfile>,
    pub display_selected: usize,
    pub display_mode: DisplayScreenMode,
    pub display_detail_field: usize, // 0=color 1=frame 2=orientation 3=ratio
    pub new_display_draft: DisplayProfileDraft,

    // Build wizard
    pub build_step: BuildStep,
    pub build_taste_idx: usize,
    pub build_display_idx: usize,
    pub build_output_dir: String,

    // Database
    pub conn: Connection,
}

impl App {
    pub fn new() -> Result<Self> {
        let default_output_dir = env::var("HOME")
            .map(|h| format!("{}/.local/share/artgg/gallery", h))
            .unwrap_or_else(|_| "~/.local/share/artgg/gallery".to_string());

        let conn = db::open()?;
        let taste_profiles = db::load_taste_profiles(&conn)?;
        let display_profiles = db::load_display_profiles(&conn)?;
        let available_keywords = db::load_keywords(&conn)?;

        Ok(Self {
            screen: Screen::Main,
            should_quit: false,
            main_selected: 0,
            taste_profiles,
            taste_selected: 0,
            taste_mode: TasteScreenMode::Browse,
            taste_detail_field: 0,
            available_keywords,
            keyword_cursor: 0,
            new_taste_draft: TasteProfileDraft::default(),
            display_profiles,
            display_selected: 0,
            display_mode: DisplayScreenMode::Browse,
            display_detail_field: 0,
            new_display_draft: DisplayProfileDraft::default(),
            build_step: BuildStep::PickTaste,
            build_taste_idx: 0,
            build_display_idx: 0,
            build_output_dir: default_output_dir,
            conn,
        })
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
                self.taste_mode = TasteScreenMode::Browse;
            }
            MainItem::DisplayProfiles => {
                self.screen = Screen::DisplayProfiles;
                self.display_mode = DisplayScreenMode::Browse;
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

    // ─── Taste profiles ──────────────────────────────────────────────────────

    fn handle_taste(&mut self, key: KeyCode) {
        match self.taste_mode.clone() {
            TasteScreenMode::Browse => match key {
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
                KeyCode::Enter => {
                    if !self.taste_profiles.is_empty() {
                        self.taste_mode = TasteScreenMode::Detail;
                        self.taste_detail_field = 0;
                    }
                }
                KeyCode::Char('a') => {
                    self.new_taste_draft = TasteProfileDraft::default();
                    self.taste_mode = TasteScreenMode::CreatingProfile;
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    if !self.taste_profiles.is_empty() {
                        let id = self.taste_profiles[self.taste_selected].id;
                        db::delete_taste_profile(&self.conn, id).expect("db delete taste");
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

            TasteScreenMode::Detail => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.taste_detail_field > 0 {
                        self.taste_detail_field -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.taste_detail_field < 4 {
                        self.taste_detail_field += 1;
                    }
                }
                KeyCode::Enter => match self.taste_detail_field {
                    0 => {
                        let val = self.taste_profiles[self.taste_selected]
                            .date_start
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        self.taste_mode = TasteScreenMode::EditingDate(val);
                    }
                    1 => {
                        let val = self.taste_profiles[self.taste_selected]
                            .date_end
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        self.taste_mode = TasteScreenMode::EditingDate(val);
                    }
                    2 => self.toggle_public_domain(),
                    3 => {
                        self.taste_mode = TasteScreenMode::SelectingKeywords;
                        self.keyword_cursor = 0;
                    }
                    _ => {} // 4 = artists, no-op
                },
                KeyCode::Char('e') => match self.taste_detail_field {
                    0 => {
                        let val = self.taste_profiles[self.taste_selected]
                            .date_start
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        self.taste_mode = TasteScreenMode::EditingDate(val);
                    }
                    1 => {
                        let val = self.taste_profiles[self.taste_selected]
                            .date_end
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        self.taste_mode = TasteScreenMode::EditingDate(val);
                    }
                    _ => {}
                },
                KeyCode::Char(' ') => {
                    if self.taste_detail_field == 2 {
                        self.toggle_public_domain();
                    }
                }
                KeyCode::Esc => {
                    self.taste_mode = TasteScreenMode::Browse;
                }
                _ => {}
            },

            TasteScreenMode::EditingDate(mut buf) => match key {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    buf.push(c);
                    self.taste_mode = TasteScreenMode::EditingDate(buf);
                }
                KeyCode::Char('-') if buf.is_empty() => {
                    buf.push('-');
                    self.taste_mode = TasteScreenMode::EditingDate(buf);
                }
                KeyCode::Backspace => {
                    buf.pop();
                    self.taste_mode = TasteScreenMode::EditingDate(buf);
                }
                KeyCode::Enter => {
                    let value: Option<i64> = if buf.is_empty() { None } else { buf.parse().ok() };
                    let idx = self.taste_selected;
                    match self.taste_detail_field {
                        0 => self.taste_profiles[idx].date_start = value,
                        1 => self.taste_profiles[idx].date_end = value,
                        _ => {}
                    }
                    let (id, ds, de, pd) = {
                        let p = &self.taste_profiles[idx];
                        (p.id, p.date_start, p.date_end, p.is_public_domain)
                    };
                    db::update_taste_profile_fields(&self.conn, id, ds, de, pd)
                        .expect("db update taste fields");
                    self.taste_mode = TasteScreenMode::Detail;
                }
                KeyCode::Esc => {
                    self.taste_mode = TasteScreenMode::Detail;
                }
                _ => {}
            },

            TasteScreenMode::SelectingKeywords => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.keyword_cursor > 0 {
                        self.keyword_cursor -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.available_keywords.is_empty()
                        && self.keyword_cursor < self.available_keywords.len() - 1
                    {
                        self.keyword_cursor += 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.toggle_keyword();
                }
                KeyCode::Esc => {
                    self.taste_mode = TasteScreenMode::Detail;
                }
                _ => {}
            },

            // ── Creating flow ──────────────────────────────────────────────

            TasteScreenMode::CreatingProfile => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.new_taste_draft.current_field > 0 {
                        self.new_taste_draft.current_field -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.new_taste_draft.current_field < 4 {
                        self.new_taste_draft.current_field += 1;
                    }
                }
                KeyCode::Enter => match self.new_taste_draft.current_field {
                    0 => {
                        let val = self.new_taste_draft.date_start
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        self.taste_mode = TasteScreenMode::CreatingEditDate(val);
                    }
                    1 => {
                        let val = self.new_taste_draft.date_end
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        self.taste_mode = TasteScreenMode::CreatingEditDate(val);
                    }
                    2 => {
                        self.new_taste_draft.is_public_domain =
                            !self.new_taste_draft.is_public_domain;
                    }
                    3 => {
                        self.keyword_cursor = 0;
                        self.taste_mode = TasteScreenMode::CreatingSelectKeywords;
                    }
                    4 => {
                        let start = self.new_taste_draft.name.clone();
                        self.taste_mode = TasteScreenMode::CreatingName(start);
                    }
                    _ => {}
                },
                KeyCode::Char(' ') => {
                    if self.new_taste_draft.current_field == 2 {
                        self.new_taste_draft.is_public_domain =
                            !self.new_taste_draft.is_public_domain;
                    }
                }
                KeyCode::Esc => {
                    self.taste_mode = TasteScreenMode::Browse;
                }
                _ => {}
            },

            TasteScreenMode::CreatingEditDate(mut buf) => match key {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    buf.push(c);
                    self.taste_mode = TasteScreenMode::CreatingEditDate(buf);
                }
                KeyCode::Char('-') if buf.is_empty() => {
                    buf.push('-');
                    self.taste_mode = TasteScreenMode::CreatingEditDate(buf);
                }
                KeyCode::Backspace => {
                    buf.pop();
                    self.taste_mode = TasteScreenMode::CreatingEditDate(buf);
                }
                KeyCode::Enter => {
                    let value: Option<i64> = if buf.is_empty() { None } else { buf.parse().ok() };
                    match self.new_taste_draft.current_field {
                        0 => self.new_taste_draft.date_start = value,
                        1 => self.new_taste_draft.date_end = value,
                        _ => {}
                    }
                    self.taste_mode = TasteScreenMode::CreatingProfile;
                }
                KeyCode::Esc => {
                    self.taste_mode = TasteScreenMode::CreatingProfile;
                }
                _ => {}
            },

            TasteScreenMode::CreatingSelectKeywords => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.keyword_cursor > 0 {
                        self.keyword_cursor -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.available_keywords.is_empty()
                        && self.keyword_cursor < self.available_keywords.len() - 1
                    {
                        self.keyword_cursor += 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.toggle_keyword_in_draft();
                }
                KeyCode::Esc => {
                    self.taste_mode = TasteScreenMode::CreatingProfile;
                }
                _ => {}
            },

            TasteScreenMode::CreatingName(mut buf) => match key {
                KeyCode::Char(c) => {
                    buf.push(c);
                    self.taste_mode = TasteScreenMode::CreatingName(buf);
                }
                KeyCode::Backspace => {
                    buf.pop();
                    self.taste_mode = TasteScreenMode::CreatingName(buf);
                }
                KeyCode::Enter => {
                    if !buf.is_empty() {
                        let date_start = self.new_taste_draft.date_start;
                        let date_end = self.new_taste_draft.date_end;
                        let is_public_domain = self.new_taste_draft.is_public_domain;
                        let keywords = std::mem::take(&mut self.new_taste_draft.keywords);
                        let id = db::insert_taste_profile(
                            &self.conn,
                            &buf,
                            date_start,
                            date_end,
                            is_public_domain,
                        )
                        .expect("db insert taste");
                        for kw_val in &keywords {
                            if let Some((kw_id, _)) =
                                self.available_keywords.iter().find(|(_, v)| v == kw_val)
                            {
                                let kw_id = *kw_id;
                                db::add_taste_profile_keyword(&self.conn, id, kw_id)
                                    .expect("db add keyword");
                            }
                        }
                        self.taste_profiles.push(TasteProfile {
                            id,
                            name: buf,
                            date_start,
                            date_end,
                            is_public_domain,
                            keywords,
                        });
                        self.taste_selected = self.taste_profiles.len() - 1;
                        self.taste_mode = TasteScreenMode::Browse;
                    }
                }
                KeyCode::Esc => {
                    // Save partial name so it's restored if user returns
                    self.new_taste_draft.name = buf;
                    self.new_taste_draft.current_field = 4;
                    self.taste_mode = TasteScreenMode::CreatingProfile;
                }
                _ => {}
            },
        }
    }

    fn toggle_public_domain(&mut self) {
        let idx = self.taste_selected;
        self.taste_profiles[idx].is_public_domain = !self.taste_profiles[idx].is_public_domain;
        let (id, ds, de, pd) = {
            let p = &self.taste_profiles[idx];
            (p.id, p.date_start, p.date_end, p.is_public_domain)
        };
        db::update_taste_profile_fields(&self.conn, id, ds, de, pd).expect("db update");
    }

    fn toggle_keyword(&mut self) {
        if self.available_keywords.is_empty() {
            return;
        }
        let (kw_id, kw_val) = self.available_keywords[self.keyword_cursor].clone();
        let idx = self.taste_selected;
        if self.taste_profiles[idx].keywords.contains(&kw_val) {
            self.taste_profiles[idx].keywords.retain(|k| k != &kw_val);
            let profile_id = self.taste_profiles[idx].id;
            db::remove_taste_profile_keyword(&self.conn, profile_id, kw_id)
                .expect("db remove keyword");
        } else if self.taste_profiles[idx].keywords.len() < 10 {
            self.taste_profiles[idx].keywords.push(kw_val);
            let profile_id = self.taste_profiles[idx].id;
            db::add_taste_profile_keyword(&self.conn, profile_id, kw_id)
                .expect("db add keyword");
        }
    }

    fn toggle_keyword_in_draft(&mut self) {
        if self.available_keywords.is_empty() {
            return;
        }
        let (_, kw_val) = self.available_keywords[self.keyword_cursor].clone();
        if self.new_taste_draft.keywords.contains(&kw_val) {
            self.new_taste_draft.keywords.retain(|k| k != &kw_val);
        } else if self.new_taste_draft.keywords.len() < 10 {
            self.new_taste_draft.keywords.push(kw_val);
        }
    }

    // ─── Display profiles ─────────────────────────────────────────────────────

    fn handle_display(&mut self, key: KeyCode) {
        match self.display_mode.clone() {
            DisplayScreenMode::Browse => match key {
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
                KeyCode::Enter => {
                    if !self.display_profiles.is_empty() {
                        self.display_mode = DisplayScreenMode::Detail;
                        self.display_detail_field = 0;
                    }
                }
                KeyCode::Char('a') => {
                    self.new_display_draft = DisplayProfileDraft::default();
                    self.display_mode = DisplayScreenMode::CreatingProfile;
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    if !self.display_profiles.is_empty() {
                        let id = self.display_profiles[self.display_selected].id;
                        db::delete_display_profile(&self.conn, id).expect("db delete display");
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

            DisplayScreenMode::Detail => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.display_detail_field > 0 {
                        self.display_detail_field -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.display_detail_field < 3 {
                        self.display_detail_field += 1;
                    }
                }
                KeyCode::Enter => match self.display_detail_field {
                    0 => {
                        let val = self.display_profiles[self.display_selected]
                            .wallpaper_color
                            .clone();
                        self.display_mode = DisplayScreenMode::EditingText(val);
                    }
                    1 => {} // frame style — disabled
                    2 => self.toggle_orientation(),
                    3 => {
                        let val = self.display_profiles[self.display_selected]
                            .aspect_ratio
                            .clone();
                        self.display_mode = DisplayScreenMode::EditingText(val);
                    }
                    _ => {}
                },
                KeyCode::Char('e') => match self.display_detail_field {
                    0 => {
                        let val = self.display_profiles[self.display_selected]
                            .wallpaper_color
                            .clone();
                        self.display_mode = DisplayScreenMode::EditingText(val);
                    }
                    3 => {
                        let val = self.display_profiles[self.display_selected]
                            .aspect_ratio
                            .clone();
                        self.display_mode = DisplayScreenMode::EditingText(val);
                    }
                    _ => {}
                },
                KeyCode::Char(' ') => {
                    if self.display_detail_field == 2 {
                        self.toggle_orientation();
                    }
                }
                KeyCode::Esc => {
                    self.display_mode = DisplayScreenMode::Browse;
                }
                _ => {}
            },

            DisplayScreenMode::EditingText(mut buf) => match key {
                KeyCode::Char(c) => {
                    buf.push(c);
                    self.display_mode = DisplayScreenMode::EditingText(buf);
                }
                KeyCode::Backspace => {
                    buf.pop();
                    self.display_mode = DisplayScreenMode::EditingText(buf);
                }
                KeyCode::Enter => {
                    let idx = self.display_selected;
                    match self.display_detail_field {
                        0 => self.display_profiles[idx].wallpaper_color = buf.clone(),
                        3 => self.display_profiles[idx].aspect_ratio = buf.clone(),
                        _ => {}
                    }
                    let (id, color, frame, orient, ratio) = {
                        let p = &self.display_profiles[idx];
                        (
                            p.id,
                            p.wallpaper_color.clone(),
                            p.frame_style.clone(),
                            p.orientation.clone(),
                            p.aspect_ratio.clone(),
                        )
                    };
                    db::update_display_profile_fields(
                        &self.conn,
                        id,
                        &color,
                        &frame,
                        &orient,
                        &ratio,
                    )
                    .expect("db update display");
                    self.display_mode = DisplayScreenMode::Detail;
                }
                KeyCode::Esc => {
                    self.display_mode = DisplayScreenMode::Detail;
                }
                _ => {}
            },

            // ── Creating flow ──────────────────────────────────────────────

            DisplayScreenMode::CreatingProfile => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.new_display_draft.current_field > 0 {
                        self.new_display_draft.current_field -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.new_display_draft.current_field < 4 {
                        self.new_display_draft.current_field += 1;
                    }
                }
                KeyCode::Enter => match self.new_display_draft.current_field {
                    0 => {
                        let val = self.new_display_draft.wallpaper_color.clone();
                        self.display_mode = DisplayScreenMode::CreatingEditText(val);
                    }
                    1 => {} // frame style — disabled
                    2 => {
                        let o = &self.new_display_draft.orientation;
                        self.new_display_draft.orientation =
                            if o == "horizontal" { "vertical" } else { "horizontal" }.to_string();
                    }
                    3 => {
                        let val = self.new_display_draft.aspect_ratio.clone();
                        self.display_mode = DisplayScreenMode::CreatingEditText(val);
                    }
                    4 => {
                        let default_name = self.display_default_name();
                        let start = if self.new_display_draft.name.is_empty() {
                            default_name
                        } else {
                            self.new_display_draft.name.clone()
                        };
                        self.display_mode = DisplayScreenMode::CreatingName(start);
                    }
                    _ => {}
                },
                KeyCode::Char(' ') => {
                    if self.new_display_draft.current_field == 2 {
                        let o = &self.new_display_draft.orientation;
                        self.new_display_draft.orientation =
                            if o == "horizontal" { "vertical" } else { "horizontal" }.to_string();
                    }
                }
                KeyCode::Esc => {
                    self.display_mode = DisplayScreenMode::Browse;
                }
                _ => {}
            },

            DisplayScreenMode::CreatingEditText(mut buf) => match key {
                KeyCode::Char(c) => {
                    buf.push(c);
                    self.display_mode = DisplayScreenMode::CreatingEditText(buf);
                }
                KeyCode::Backspace => {
                    buf.pop();
                    self.display_mode = DisplayScreenMode::CreatingEditText(buf);
                }
                KeyCode::Enter => {
                    match self.new_display_draft.current_field {
                        0 => self.new_display_draft.wallpaper_color = buf,
                        3 => self.new_display_draft.aspect_ratio = buf,
                        _ => {}
                    }
                    self.display_mode = DisplayScreenMode::CreatingProfile;
                }
                KeyCode::Esc => {
                    self.display_mode = DisplayScreenMode::CreatingProfile;
                }
                _ => {}
            },

            DisplayScreenMode::CreatingName(mut buf) => match key {
                KeyCode::Char(c) => {
                    buf.push(c);
                    self.display_mode = DisplayScreenMode::CreatingName(buf);
                }
                KeyCode::Backspace => {
                    buf.pop();
                    self.display_mode = DisplayScreenMode::CreatingName(buf);
                }
                KeyCode::Enter => {
                    if !buf.is_empty() {
                        let color = self.new_display_draft.wallpaper_color.clone();
                        let frame = self.new_display_draft.frame_style.clone();
                        let orient = self.new_display_draft.orientation.clone();
                        let ratio = self.new_display_draft.aspect_ratio.clone();
                        let id = db::insert_display_profile(
                            &self.conn,
                            &buf,
                            &color,
                            &frame,
                            &orient,
                            &ratio,
                        )
                        .expect("db insert display");
                        self.display_profiles.push(DisplayProfile {
                            id,
                            name: buf,
                            wallpaper_color: color,
                            frame_style: frame,
                            orientation: orient,
                            aspect_ratio: ratio,
                        });
                        self.display_selected = self.display_profiles.len() - 1;
                        self.display_mode = DisplayScreenMode::Browse;
                    }
                }
                KeyCode::Esc => {
                    self.new_display_draft.name = buf;
                    self.new_display_draft.current_field = 4;
                    self.display_mode = DisplayScreenMode::CreatingProfile;
                }
                _ => {}
            },
        }
    }

    fn toggle_orientation(&mut self) {
        let idx = self.display_selected;
        {
            let p = &mut self.display_profiles[idx];
            p.orientation = if p.orientation == "horizontal" {
                "vertical".to_string()
            } else {
                "horizontal".to_string()
            };
        }
        let (id, color, frame, orient, ratio) = {
            let p = &self.display_profiles[idx];
            (
                p.id,
                p.wallpaper_color.clone(),
                p.frame_style.clone(),
                p.orientation.clone(),
                p.aspect_ratio.clone(),
            )
        };
        db::update_display_profile_fields(&self.conn, id, &color, &frame, &orient, &ratio)
            .expect("db update display orientation");
    }

    /// Generate a human-readable default name from the current display draft.
    fn display_default_name(&self) -> String {
        let o = &self.new_display_draft.orientation;
        let o_cap: String = {
            let mut chars = o.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        };
        format!("{} {}", o_cap, self.new_display_draft.aspect_ratio)
    }

    // ─── Build wizard ─────────────────────────────────────────────────────────

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
