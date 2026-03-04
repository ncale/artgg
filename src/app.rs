use anyhow::Result;
use crossterm::event::KeyCode;
use rusqlite::Connection;
use std::env;
use std::sync::mpsc::Receiver;

use crate::build::{self, BuildParams};
use crate::collection;
use crate::db;

// ---------------------------------------------------------------------------
// Build progress messages (sent from build thread → main thread)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum BuildMessage {
    Phase(String),
    Progress { current: usize, total: usize, message: String },
    Done { produced: usize, skipped: usize, output_dir: String },
    Error(String),
}

// ---------------------------------------------------------------------------
// Screen / mode enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Main,
    TasteProfiles,
    DisplayProfiles,
    Build,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TasteScreenMode {
    Browse,
    Detail,
    EditingDate(String),
    SelectingKeywords,
    CreatingProfile,
    CreatingEditDate(String),
    CreatingSelectKeywords,
    CreatingName(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayScreenMode {
    Browse,
    Detail,
    EditingText(String),
    CreatingProfile,
    CreatingEditText(String),
    CreatingName(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildStep {
    PickTaste,
    PickDisplay,
    PickOutputDir,
    PickCount,
    Running,
    Done,
}

// ---------------------------------------------------------------------------
// Profile structs
// ---------------------------------------------------------------------------

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
    pub orientation: String,
    pub canvas_width: u32,
    pub canvas_height: u32,
}

// ---------------------------------------------------------------------------
// Draft structs (used while creating a new profile)
// ---------------------------------------------------------------------------

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
            is_public_domain: true,
            keywords: vec![],
            name: String::new(),
            current_field: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DisplayProfileDraft {
    pub wallpaper_color: String,
    pub frame_style: String,
    pub orientation: String,
    pub canvas_width: String,
    pub canvas_height: String,
    pub name: String,
    pub current_field: usize, // 0=color 1=frame 2=orientation 3=width 4=height 5=name
}

impl Default for DisplayProfileDraft {
    fn default() -> Self {
        Self {
            wallpaper_color: "#1A1A2E".to_string(),
            frame_style: String::new(),
            orientation: "horizontal".to_string(),
            canvas_width: "1920".to_string(),
            canvas_height: "1080".to_string(),
            name: String::new(),
            current_field: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Main menu
// ---------------------------------------------------------------------------

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
            MainItem::TasteProfiles  => "Taste Profiles",
            MainItem::DisplayProfiles => "Display Profiles",
            MainItem::Build          => "Build",
            MainItem::Prune          => "Prune",
            MainItem::Exit           => "Exit",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MainItem::TasteProfiles  => "Manage your art taste profiles (subjects, styles, periods)",
            MainItem::DisplayProfiles => "Manage your display profiles (resolution, color, frame)",
            MainItem::Build          => "Build a wallpaper gallery by picking a taste + display profile",
            MainItem::Prune          => "Remove old images based on retention limits (coming soon)",
            MainItem::Exit           => "Exit artgg",
        }
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, MainItem::Prune)
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,

    // Main menu
    pub main_selected: usize,

    // Taste profiles
    pub taste_profiles: Vec<TasteProfile>,
    pub taste_selected: usize,
    pub taste_mode: TasteScreenMode,
    pub taste_detail_field: usize,
    pub available_keywords: Vec<(i64, String)>,
    pub keyword_cursor: usize,
    pub new_taste_draft: TasteProfileDraft,

    // Display profiles
    pub display_profiles: Vec<DisplayProfile>,
    pub display_selected: usize,
    pub display_mode: DisplayScreenMode,
    pub display_detail_field: usize, // 0=color 1=frame 2=orientation 3=width 4=height
    pub new_display_draft: DisplayProfileDraft,

    // Build wizard
    pub build_step: BuildStep,
    pub build_taste_idx: usize,
    pub build_display_idx: usize,
    pub build_output_dir: String,
    pub build_count_str: String,

    // Build progress (Running / Done state)
    pub build_rx: Option<Receiver<BuildMessage>>,
    pub build_log: Vec<String>,
    pub build_progress: (usize, usize), // (current, total)
    pub build_phase: String,
    pub build_produced: usize,
    pub build_skipped: usize,
    pub build_done_dir: String,

    // Database (user data)
    pub conn: Connection,
}

impl App {
    pub fn new() -> Result<Self> {
        let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
        let default_output_dir = format!("{}/.local/share/artgg/gallery", home);

        let conn = db::open()?;
        let taste_profiles    = db::load_taste_profiles(&conn)?;
        let display_profiles  = db::load_display_profiles(&conn)?;
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
            build_count_str: "20".to_string(),

            build_rx: None,
            build_log: Vec::new(),
            build_progress: (0, 0),
            build_phase: String::new(),
            build_produced: 0,
            build_skipped: 0,
            build_done_dir: String::new(),

            conn,
        })
    }

    // ── Channel polling (called every loop tick) ───────────────────────────

    pub fn poll_build_messages(&mut self) {
        loop {
            let msg = match self.build_rx.as_ref() {
                Some(rx) => match rx.try_recv() {
                    Ok(m) => m,
                    Err(_) => break,
                },
                None => break,
            };

            match msg {
                BuildMessage::Phase(s) => {
                    self.build_phase = s.clone();
                    self.push_log(s);
                }
                BuildMessage::Progress { current, total, message } => {
                    self.build_progress = (current, total);
                    self.push_log(message);
                }
                BuildMessage::Done { produced, skipped, output_dir } => {
                    self.build_produced  = produced;
                    self.build_skipped   = skipped;
                    self.build_done_dir  = output_dir;
                    self.build_step = BuildStep::Done;
                    self.build_rx = None;
                }
                BuildMessage::Error(e) => {
                    self.push_log(format!("ERROR: {}", e));
                    self.build_step = BuildStep::Done;
                    self.build_rx = None;
                }
            }
        }
    }

    fn push_log(&mut self, msg: String) {
        self.build_log.push(msg);
        if self.build_log.len() > 200 {
            self.build_log.drain(..100);
        }
    }

    // ── Key dispatcher ─────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyCode) {
        match self.screen {
            Screen::Main          => self.handle_main(key),
            Screen::TasteProfiles  => self.handle_taste(key),
            Screen::DisplayProfiles => self.handle_display(key),
            Screen::Build          => self.handle_build(key),
        }
    }

    // ── Main menu ──────────────────────────────────────────────────────────

    fn handle_main(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Char('k')   => self.main_move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.main_move_down(),
            KeyCode::Enter                     => self.main_activate(),
            KeyCode::Char('q') | KeyCode::Esc  => self.should_quit = true,
            _ => {}
        }
    }

    fn main_move_up(&mut self) {
        let items = MainItem::ALL;
        let mut idx = if self.main_selected == 0 { items.len() - 1 } else { self.main_selected - 1 };
        while items[idx].is_disabled() {
            idx = if idx == 0 { items.len() - 1 } else { idx - 1 };
        }
        self.main_selected = idx;
    }

    fn main_move_down(&mut self) {
        let items = MainItem::ALL;
        let mut idx = (self.main_selected + 1) % items.len();
        while items[idx].is_disabled() { idx = (idx + 1) % items.len(); }
        self.main_selected = idx;
    }

    fn main_activate(&mut self) {
        match MainItem::ALL[self.main_selected] {
            MainItem::TasteProfiles  => {
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
            MainItem::Exit  => self.should_quit = true,
        }
    }

    // ── Taste profiles ─────────────────────────────────────────────────────

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
                        if self.taste_selected > 0 && self.taste_selected >= self.taste_profiles.len() {
                            self.taste_selected = self.taste_profiles.len() - 1;
                        }
                    }
                }
                KeyCode::Esc => { self.screen = Screen::Main; }
                _ => {}
            },

            TasteScreenMode::Detail => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.taste_detail_field > 0 { self.taste_detail_field -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.taste_detail_field < 4 { self.taste_detail_field += 1; }
                }
                KeyCode::Enter => match self.taste_detail_field {
                    0 => {
                        let val = self.taste_profiles[self.taste_selected].date_start
                            .map(|v| v.to_string()).unwrap_or_default();
                        self.taste_mode = TasteScreenMode::EditingDate(val);
                    }
                    1 => {
                        let val = self.taste_profiles[self.taste_selected].date_end
                            .map(|v| v.to_string()).unwrap_or_default();
                        self.taste_mode = TasteScreenMode::EditingDate(val);
                    }
                    2 => self.toggle_public_domain(),
                    3 => { self.taste_mode = TasteScreenMode::SelectingKeywords; self.keyword_cursor = 0; }
                    _ => {}
                },
                KeyCode::Char('e') => match self.taste_detail_field {
                    0 => {
                        let val = self.taste_profiles[self.taste_selected].date_start
                            .map(|v| v.to_string()).unwrap_or_default();
                        self.taste_mode = TasteScreenMode::EditingDate(val);
                    }
                    1 => {
                        let val = self.taste_profiles[self.taste_selected].date_end
                            .map(|v| v.to_string()).unwrap_or_default();
                        self.taste_mode = TasteScreenMode::EditingDate(val);
                    }
                    _ => {}
                },
                KeyCode::Char(' ') => {
                    if self.taste_detail_field == 2 { self.toggle_public_domain(); }
                }
                KeyCode::Esc => { self.taste_mode = TasteScreenMode::Browse; }
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
                        1 => self.taste_profiles[idx].date_end   = value,
                        _ => {}
                    }
                    let (id, ds, de, pd) = {
                        let p = &self.taste_profiles[idx];
                        (p.id, p.date_start, p.date_end, p.is_public_domain)
                    };
                    db::update_taste_profile_fields(&self.conn, id, ds, de, pd).expect("db update taste");
                    self.taste_mode = TasteScreenMode::Detail;
                }
                KeyCode::Esc => { self.taste_mode = TasteScreenMode::Detail; }
                _ => {}
            },

            TasteScreenMode::SelectingKeywords => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.keyword_cursor > 0 { self.keyword_cursor -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.available_keywords.is_empty()
                        && self.keyword_cursor < self.available_keywords.len() - 1
                    {
                        self.keyword_cursor += 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => { self.toggle_keyword(); }
                KeyCode::Esc => { self.taste_mode = TasteScreenMode::Detail; }
                _ => {}
            },

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
                        let val = self.new_taste_draft.date_start.map(|v| v.to_string()).unwrap_or_default();
                        self.taste_mode = TasteScreenMode::CreatingEditDate(val);
                    }
                    1 => {
                        let val = self.new_taste_draft.date_end.map(|v| v.to_string()).unwrap_or_default();
                        self.taste_mode = TasteScreenMode::CreatingEditDate(val);
                    }
                    2 => { self.new_taste_draft.is_public_domain = !self.new_taste_draft.is_public_domain; }
                    3 => { self.keyword_cursor = 0; self.taste_mode = TasteScreenMode::CreatingSelectKeywords; }
                    4 => {
                        let start = self.new_taste_draft.name.clone();
                        self.taste_mode = TasteScreenMode::CreatingName(start);
                    }
                    _ => {}
                },
                KeyCode::Char(' ') => {
                    if self.new_taste_draft.current_field == 2 {
                        self.new_taste_draft.is_public_domain = !self.new_taste_draft.is_public_domain;
                    }
                }
                KeyCode::Esc => { self.taste_mode = TasteScreenMode::Browse; }
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
                        1 => self.new_taste_draft.date_end   = value,
                        _ => {}
                    }
                    self.taste_mode = TasteScreenMode::CreatingProfile;
                }
                KeyCode::Esc => { self.taste_mode = TasteScreenMode::CreatingProfile; }
                _ => {}
            },

            TasteScreenMode::CreatingSelectKeywords => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.keyword_cursor > 0 { self.keyword_cursor -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.available_keywords.is_empty()
                        && self.keyword_cursor < self.available_keywords.len() - 1
                    {
                        self.keyword_cursor += 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => { self.toggle_keyword_in_draft(); }
                KeyCode::Esc => { self.taste_mode = TasteScreenMode::CreatingProfile; }
                _ => {}
            },

            TasteScreenMode::CreatingName(mut buf) => match key {
                KeyCode::Char(c) => { buf.push(c); self.taste_mode = TasteScreenMode::CreatingName(buf); }
                KeyCode::Backspace => { buf.pop(); self.taste_mode = TasteScreenMode::CreatingName(buf); }
                KeyCode::Enter => {
                    if !buf.is_empty() {
                        let date_start     = self.new_taste_draft.date_start;
                        let date_end       = self.new_taste_draft.date_end;
                        let is_public_domain = self.new_taste_draft.is_public_domain;
                        let keywords       = std::mem::take(&mut self.new_taste_draft.keywords);
                        let id = db::insert_taste_profile(
                            &self.conn, &buf, date_start, date_end, is_public_domain,
                        ).expect("db insert taste");
                        for kw_val in &keywords {
                            if let Some((kw_id, _)) = self.available_keywords.iter().find(|(_, v)| v == kw_val) {
                                db::add_taste_profile_keyword(&self.conn, id, *kw_id).expect("db add kw");
                            }
                        }
                        self.taste_profiles.push(TasteProfile {
                            id, name: buf, date_start, date_end, is_public_domain, keywords,
                        });
                        self.taste_selected = self.taste_profiles.len() - 1;
                        self.taste_mode = TasteScreenMode::Browse;
                    }
                }
                KeyCode::Esc => {
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
        if self.available_keywords.is_empty() { return; }
        let (kw_id, kw_val) = self.available_keywords[self.keyword_cursor].clone();
        let idx = self.taste_selected;
        if self.taste_profiles[idx].keywords.contains(&kw_val) {
            self.taste_profiles[idx].keywords.retain(|k| k != &kw_val);
            let pid = self.taste_profiles[idx].id;
            db::remove_taste_profile_keyword(&self.conn, pid, kw_id).expect("db rm kw");
        } else if self.taste_profiles[idx].keywords.len() < 10 {
            self.taste_profiles[idx].keywords.push(kw_val);
            let pid = self.taste_profiles[idx].id;
            db::add_taste_profile_keyword(&self.conn, pid, kw_id).expect("db add kw");
        }
    }

    fn toggle_keyword_in_draft(&mut self) {
        if self.available_keywords.is_empty() { return; }
        let (_, kw_val) = self.available_keywords[self.keyword_cursor].clone();
        if self.new_taste_draft.keywords.contains(&kw_val) {
            self.new_taste_draft.keywords.retain(|k| k != &kw_val);
        } else if self.new_taste_draft.keywords.len() < 10 {
            self.new_taste_draft.keywords.push(kw_val);
        }
    }

    // ── Display profiles ───────────────────────────────────────────────────

    fn handle_display(&mut self, key: KeyCode) {
        // Max detail field index: 4 (0=color 1=frame 2=orient 3=width 4=height)
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
                KeyCode::Esc => { self.screen = Screen::Main; }
                _ => {}
            },

            DisplayScreenMode::Detail => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.display_detail_field > 0 { self.display_detail_field -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.display_detail_field < 4 { self.display_detail_field += 1; }
                }
                KeyCode::Enter | KeyCode::Char('e') => match self.display_detail_field {
                    0 => {
                        let v = self.display_profiles[self.display_selected].wallpaper_color.clone();
                        self.display_mode = DisplayScreenMode::EditingText(v);
                    }
                    1 => {} // frame style — disabled
                    2 => self.toggle_orientation(),
                    3 => {
                        let v = self.display_profiles[self.display_selected].canvas_width.to_string();
                        self.display_mode = DisplayScreenMode::EditingText(v);
                    }
                    4 => {
                        let v = self.display_profiles[self.display_selected].canvas_height.to_string();
                        self.display_mode = DisplayScreenMode::EditingText(v);
                    }
                    _ => {}
                },
                KeyCode::Char(' ') => {
                    if self.display_detail_field == 2 { self.toggle_orientation(); }
                }
                KeyCode::Esc => { self.display_mode = DisplayScreenMode::Browse; }
                _ => {}
            },

            DisplayScreenMode::EditingText(mut buf) => match key {
                KeyCode::Char(c) => { buf.push(c); self.display_mode = DisplayScreenMode::EditingText(buf); }
                KeyCode::Backspace => { buf.pop(); self.display_mode = DisplayScreenMode::EditingText(buf); }
                KeyCode::Enter => {
                    let idx = self.display_selected;
                    match self.display_detail_field {
                        0 => self.display_profiles[idx].wallpaper_color = buf.clone(),
                        3 => {
                            if let Ok(v) = buf.parse::<u32>() {
                                self.display_profiles[idx].canvas_width = v;
                            }
                        }
                        4 => {
                            if let Ok(v) = buf.parse::<u32>() {
                                self.display_profiles[idx].canvas_height = v;
                            }
                        }
                        _ => {}
                    }
                    let p = &self.display_profiles[idx];
                    db::update_display_profile_fields(
                        &self.conn, p.id, &p.wallpaper_color, &p.frame_style,
                        &p.orientation, p.canvas_width, p.canvas_height,
                    ).expect("db update display");
                    self.display_mode = DisplayScreenMode::Detail;
                }
                KeyCode::Esc => { self.display_mode = DisplayScreenMode::Detail; }
                _ => {}
            },

            DisplayScreenMode::CreatingProfile => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.new_display_draft.current_field > 0 {
                        self.new_display_draft.current_field -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.new_display_draft.current_field < 5 {
                        self.new_display_draft.current_field += 1;
                    }
                }
                KeyCode::Enter => match self.new_display_draft.current_field {
                    0 => {
                        let v = self.new_display_draft.wallpaper_color.clone();
                        self.display_mode = DisplayScreenMode::CreatingEditText(v);
                    }
                    1 => {} // frame style — disabled
                    2 => {
                        let o = &self.new_display_draft.orientation;
                        self.new_display_draft.orientation =
                            if o == "horizontal" { "vertical" } else { "horizontal" }.to_string();
                    }
                    3 => {
                        let v = self.new_display_draft.canvas_width.clone();
                        self.display_mode = DisplayScreenMode::CreatingEditText(v);
                    }
                    4 => {
                        let v = self.new_display_draft.canvas_height.clone();
                        self.display_mode = DisplayScreenMode::CreatingEditText(v);
                    }
                    5 => {
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
                KeyCode::Esc => { self.display_mode = DisplayScreenMode::Browse; }
                _ => {}
            },

            DisplayScreenMode::CreatingEditText(mut buf) => match key {
                KeyCode::Char(c) => { buf.push(c); self.display_mode = DisplayScreenMode::CreatingEditText(buf); }
                KeyCode::Backspace => { buf.pop(); self.display_mode = DisplayScreenMode::CreatingEditText(buf); }
                KeyCode::Enter => {
                    match self.new_display_draft.current_field {
                        0 => self.new_display_draft.wallpaper_color = buf,
                        3 => self.new_display_draft.canvas_width     = buf,
                        4 => self.new_display_draft.canvas_height    = buf,
                        _ => {}
                    }
                    self.display_mode = DisplayScreenMode::CreatingProfile;
                }
                KeyCode::Esc => { self.display_mode = DisplayScreenMode::CreatingProfile; }
                _ => {}
            },

            DisplayScreenMode::CreatingName(mut buf) => match key {
                KeyCode::Char(c) => { buf.push(c); self.display_mode = DisplayScreenMode::CreatingName(buf); }
                KeyCode::Backspace => { buf.pop(); self.display_mode = DisplayScreenMode::CreatingName(buf); }
                KeyCode::Enter => {
                    if !buf.is_empty() {
                        let d = &self.new_display_draft;
                        let w = d.canvas_width.parse::<u32>().unwrap_or(1920);
                        let h = d.canvas_height.parse::<u32>().unwrap_or(1080);
                        let id = db::insert_display_profile(
                            &self.conn, &buf, &d.wallpaper_color, &d.frame_style,
                            &d.orientation, w, h,
                        ).expect("db insert display");
                        self.display_profiles.push(DisplayProfile {
                            id,
                            name: buf,
                            wallpaper_color: d.wallpaper_color.clone(),
                            frame_style: d.frame_style.clone(),
                            orientation: d.orientation.clone(),
                            canvas_width: w,
                            canvas_height: h,
                        });
                        self.display_selected = self.display_profiles.len() - 1;
                        self.display_mode = DisplayScreenMode::Browse;
                    }
                }
                KeyCode::Esc => {
                    self.new_display_draft.name = buf;
                    self.new_display_draft.current_field = 5;
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
        let p = &self.display_profiles[idx];
        db::update_display_profile_fields(
            &self.conn, p.id, &p.wallpaper_color, &p.frame_style,
            &p.orientation, p.canvas_width, p.canvas_height,
        ).expect("db update orientation");
    }

    fn display_default_name(&self) -> String {
        let d = &self.new_display_draft;
        let o_cap = {
            let mut chars = d.orientation.chars();
            match chars.next() {
                None    => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        };
        format!("{} {}x{}", o_cap, d.canvas_width, d.canvas_height)
    }

    // ── Build wizard ───────────────────────────────────────────────────────

    fn handle_build(&mut self, key: KeyCode) {
        match self.build_step {
            BuildStep::PickTaste => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.build_taste_idx > 0 { self.build_taste_idx -= 1; }
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
                KeyCode::Esc => { self.screen = Screen::Main; }
                _ => {}
            },

            BuildStep::PickDisplay => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.build_display_idx > 0 { self.build_display_idx -= 1; }
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
                KeyCode::Esc => { self.build_step = BuildStep::PickTaste; }
                _ => {}
            },

            BuildStep::PickOutputDir => match key {
                KeyCode::Char(c) => { self.build_output_dir.push(c); }
                KeyCode::Backspace => { self.build_output_dir.pop(); }
                KeyCode::Enter => { self.build_step = BuildStep::PickCount; }
                KeyCode::Esc   => { self.build_step = BuildStep::PickDisplay; }
                _ => {}
            },

            BuildStep::PickCount => match key {
                KeyCode::Char(c) if c.is_ascii_digit() => { self.build_count_str.push(c); }
                KeyCode::Backspace => { self.build_count_str.pop(); }
                KeyCode::Enter => {
                    let count = self.build_count_str.parse::<usize>().unwrap_or(20).max(1);
                    self.start_build(count);
                }
                KeyCode::Esc => { self.build_step = BuildStep::PickOutputDir; }
                _ => {}
            },

            BuildStep::Running => match key {
                // No-op while running; Ctrl-C is handled by the OS.
                _ => {}
            },

            BuildStep::Done => match key {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                    self.screen = Screen::Main;
                    self.build_step = BuildStep::PickTaste;
                    self.build_log.clear();
                    self.build_phase.clear();
                }
                _ => {}
            },
        }
    }

    fn start_build(&mut self, count: usize) {
        let home = env::var("HOME").unwrap_or_default();

        let collection_db = collection::find_collection_db()
            .unwrap_or_else(|| "./assets/collection.db".to_string());
        let cache_dir    = format!("{}/.cache/artgg/images", home);
        let artgg_db_path = db::db_path();

        // Guard: need valid profile indices.
        if self.taste_profiles.is_empty() || self.display_profiles.is_empty() { return; }

        let taste   = self.taste_profiles[self.build_taste_idx].clone();
        let display = self.display_profiles[self.build_display_idx].clone();

        let params = BuildParams {
            taste,
            display,
            output_dir: self.build_output_dir.clone(),
            count,
            collection_db_path: collection_db,
            cache_dir,
            artgg_db_path,
        };

        let (tx, rx) = std::sync::mpsc::channel();
        self.build_rx = Some(rx);
        self.build_log.clear();
        self.build_progress = (0, count);
        self.build_phase = "Starting…".to_string();
        self.build_produced = 0;
        self.build_skipped  = 0;
        self.build_done_dir = self.build_output_dir.clone();
        self.build_step = BuildStep::Running;

        std::thread::spawn(move || {
            build::run(params, tx);
        });
    }
}
