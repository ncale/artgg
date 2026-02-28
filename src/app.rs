use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuItem {
    Init,
    Fetch,
    Build,
    Prune,
    Quit,
}

impl MenuItem {
    pub const ALL: &'static [MenuItem] = &[
        MenuItem::Init,
        MenuItem::Fetch,
        MenuItem::Build,
        MenuItem::Prune,
        MenuItem::Quit,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            MenuItem::Init => "Init",
            MenuItem::Fetch => "Fetch",
            MenuItem::Build => "Build",
            MenuItem::Prune => "Prune",
            MenuItem::Quit => "Quit",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MenuItem::Init => "Create config.toml in your config directory",
            MenuItem::Fetch => "Download artwork from APIs into the local pool",
            MenuItem::Build => "Render a wallpaper with frame and placard",
            MenuItem::Prune => "Remove old images based on retention limits",
            MenuItem::Quit => "Exit artgg",
        }
    }
}

pub struct App {
    pub selected: usize,
    pub status: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            selected: 0,
            status: None,
            should_quit: false,
        }
    }

    pub fn selected_item(&self) -> MenuItem {
        MenuItem::ALL[self.selected]
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = MenuItem::ALL.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1) % MenuItem::ALL.len();
    }

    pub fn confirm(&mut self) -> Result<()> {
        match self.selected_item() {
            MenuItem::Quit => self.should_quit = true,
            item => {
                self.status = Some(format!(
                    "'{}' is not yet implemented.",
                    item.label()
                ));
            }
        }
        Ok(())
    }
}
