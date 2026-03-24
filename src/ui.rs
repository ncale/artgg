use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, BuildStep, DisplayScreenMode, MainItem, Screen, TasteScreenMode};

pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Main           => draw_main(frame, app),
        Screen::TasteProfiles  => draw_taste_profiles(frame, app),
        Screen::DisplayProfiles => draw_display_profiles(frame, app),
        Screen::Build          => draw_build(frame, app),
    }
}

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

fn base_layout(frame: &Frame) -> (Rect, Rect, Rect) {
    let area = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}

fn render_header(frame: &mut Frame, area: Rect, subtitle: &str) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("art", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("gg",  Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(format!("  ·  {}", subtitle)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame, area: Rect, hints: &[(&str, &str)]) {
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 { spans.push(Span::raw("   ")); }
        spans.push(Span::styled(format!(" {} ", key), Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(desc.to_string()));
    }
    let footer = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(footer, area);
}

// ---------------------------------------------------------------------------
// Main screen
// ---------------------------------------------------------------------------

fn draw_main(frame: &mut Frame, app: &App) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Classical artwork wallpaper generator");

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(0)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    let items: Vec<ListItem> = MainItem::ALL.iter().map(|item| {
        if item.is_disabled() {
            ListItem::new(item.label()).style(Style::default().fg(Color::DarkGray))
        } else {
            ListItem::new(item.label())
        }
    }).collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.main_selected));

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)).title(" Menu "))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, body[0], &mut list_state);

    let selected_item = MainItem::ALL[app.main_selected];
    let description_text = main_item_description(selected_item, &app.cache_size_label);
    let detail = Paragraph::new(description_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" {} ", selected_item.label())));
    frame.render_widget(detail, body[1]);

    render_footer(frame, footer_area, &[("↑↓", "navigate"), ("Enter", "select"), ("q", "quit")]);
}

// ---------------------------------------------------------------------------
// Taste Profiles
// ---------------------------------------------------------------------------

fn draw_taste_profiles(frame: &mut Frame, app: &App) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Taste Profiles");

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    // Left: profile list
    let left_items: Vec<ListItem> = if app.taste_profiles.is_empty() {
        vec![ListItem::new("(none)").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.taste_profiles.iter().map(|p| ListItem::new(p.name.as_str())).collect()
    };
    let mut left_state = ListState::default();
    if !app.taste_profiles.is_empty() { left_state.select(Some(app.taste_selected)); }
    let left_list = List::new(left_items)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)).title(" Taste Profiles "))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    frame.render_stateful_widget(left_list, body[0], &mut left_state);

    // Right: mode-dependent panel
    match &app.taste_mode {
        TasteScreenMode::Browse => {
            if app.taste_profiles.is_empty() {
                frame.render_widget(
                    empty_actions_panel(&[("a", "add profile"), ("Esc", "back to menu")]),
                    body[1],
                );
            } else {
                let p = &app.taste_profiles[app.taste_selected];
                let items = build_taste_detail_items(p.date_start, p.date_end, p.is_public_domain, p.departments.len(), None, "");
                let list = List::new(items).block(
                    Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray)).title(format!(" {} ", p.name))
                );
                frame.render_widget(list, body[1]);
            }
        }

        TasteScreenMode::Detail => {
            let p = &app.taste_profiles[app.taste_selected];
            let items = build_taste_detail_items(p.date_start, p.date_end, p.is_public_domain, p.departments.len(), None, "");
            let mut state = ListState::default(); state.select(Some(app.taste_detail_field));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(format!(" {} ", p.name)))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        TasteScreenMode::EditingDate(buf) => {
            let p = &app.taste_profiles[app.taste_selected];
            let items = build_taste_detail_items(p.date_start, p.date_end, p.is_public_domain, p.departments.len(), Some(app.taste_detail_field), buf);
            let mut state = ListState::default(); state.select(Some(app.taste_detail_field));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(format!(" {} ", p.name)))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        TasteScreenMode::SelectingDepartments => {
            let p = &app.taste_profiles[app.taste_selected];
            render_department_picker(frame, body[1], &app.available_departments, &p.departments, app.department_cursor);
        }

        TasteScreenMode::CreatingProfile => {
            let d = &app.new_taste_draft;
            let items = build_taste_creating_items(d.date_start, d.date_end, d.is_public_domain, d.departments.len(), &d.name, None, "");
            let mut state = ListState::default(); state.select(Some(d.current_field));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(" New Taste Profile "))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        TasteScreenMode::CreatingEditDate(buf) => {
            let d = &app.new_taste_draft;
            let items = build_taste_creating_items(d.date_start, d.date_end, d.is_public_domain, d.departments.len(), &d.name, Some(d.current_field), buf);
            let mut state = ListState::default(); state.select(Some(d.current_field));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(" New Taste Profile "))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        TasteScreenMode::CreatingSelectDepartments => {
            render_department_picker(frame, body[1], &app.available_departments, &app.new_taste_draft.departments, app.department_cursor);
        }

        TasteScreenMode::CreatingName(buf) => {
            let d = &app.new_taste_draft;
            let items = build_taste_creating_items(d.date_start, d.date_end, d.is_public_domain, d.departments.len(), buf, Some(4), buf);
            let mut state = ListState::default(); state.select(Some(4));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(" New Taste Profile "))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }
    }

    let dept_count = match &app.taste_mode {
        TasteScreenMode::SelectingDepartments => {
            if !app.taste_profiles.is_empty() { app.taste_profiles[app.taste_selected].departments.len() } else { 0 }
        }
        TasteScreenMode::CreatingSelectDepartments => app.new_taste_draft.departments.len(),
        _ => 0,
    };
    let toggle_hint = format!("toggle ({} selected)", dept_count);
    let footer_hints: Vec<(&str, &str)> = match &app.taste_mode {
        TasteScreenMode::Browse if app.taste_profiles.is_empty() => vec![("a", "add"), ("Esc", "back")],
        TasteScreenMode::Browse => vec![("↑↓", "select"), ("Enter", "edit"), ("a", "add"), ("d", "delete"), ("Esc", "back")],
        TasteScreenMode::Detail => vec![("↑↓", "navigate"), ("Enter", "edit"), ("Esc", "back")],
        TasteScreenMode::EditingDate(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
        TasteScreenMode::SelectingDepartments => vec![("↑↓", "navigate"), ("Space", toggle_hint.as_str()), ("Esc", "done")],
        TasteScreenMode::CreatingProfile => vec![("↑↓", "navigate"), ("Enter", "select"), ("Esc", "cancel")],
        TasteScreenMode::CreatingEditDate(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
        TasteScreenMode::CreatingSelectDepartments => vec![("↑↓", "navigate"), ("Space", toggle_hint.as_str()), ("Esc", "done")],
        TasteScreenMode::CreatingName(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
    };
    render_footer(frame, footer_area, &footer_hints);
}

// ---------------------------------------------------------------------------
// Display Profiles
// ---------------------------------------------------------------------------

fn draw_display_profiles(frame: &mut Frame, app: &App) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Display Profiles");

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    // Left: profile list
    let left_items: Vec<ListItem> = if app.display_profiles.is_empty() {
        vec![ListItem::new("(none)").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.display_profiles.iter().map(|p| ListItem::new(p.name.as_str())).collect()
    };
    let mut left_state = ListState::default();
    if !app.display_profiles.is_empty() { left_state.select(Some(app.display_selected)); }
    let left_list = List::new(left_items)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)).title(" Display Profiles "))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    frame.render_stateful_widget(left_list, body[0], &mut left_state);

    // Right: mode-dependent panel
    match &app.display_mode {
        DisplayScreenMode::Browse => {
            if app.display_profiles.is_empty() {
                frame.render_widget(
                    empty_actions_panel(&[("a", "add profile"), ("Esc", "back to menu")]),
                    body[1],
                );
            } else {
                let p = &app.display_profiles[app.display_selected];
                let items = build_display_detail_items(
                    &p.wallpaper_color, &p.orientation, p.canvas_width, p.canvas_height,
                    &p.placard_color, &p.placard_text_color, p.placard_opacity, None, "");
                let list = List::new(items).block(
                    Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray)).title(format!(" {} ", p.name))
                );
                frame.render_widget(list, body[1]);
            }
        }

        DisplayScreenMode::Detail => {
            let p = &app.display_profiles[app.display_selected];
            let items = build_display_detail_items(
                &p.wallpaper_color, &p.orientation, p.canvas_width, p.canvas_height,
                &p.placard_color, &p.placard_text_color, p.placard_opacity, None, "");
            let mut state = ListState::default(); state.select(Some(app.display_detail_field));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(format!(" {} ", p.name)))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        DisplayScreenMode::EditingText(buf) => {
            let p = &app.display_profiles[app.display_selected];
            let items = build_display_detail_items(
                &p.wallpaper_color, &p.orientation, p.canvas_width, p.canvas_height,
                &p.placard_color, &p.placard_text_color, p.placard_opacity,
                Some(app.display_detail_field), buf);
            let mut state = ListState::default(); state.select(Some(app.display_detail_field));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(format!(" {} ", p.name)))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        DisplayScreenMode::CreatingProfile => {
            let d = &app.new_display_draft;
            let w = d.canvas_width.parse::<u32>().unwrap_or(1920);
            let h = d.canvas_height.parse::<u32>().unwrap_or(1080);
            let op = d.placard_opacity.parse::<u32>().unwrap_or(90);
            let items = build_display_creating_items(
                &d.wallpaper_color, &d.orientation, w, h,
                &d.placard_color, &d.placard_text_color, op, &d.name, None, "");
            let mut state = ListState::default(); state.select(Some(d.current_field));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(" New Display Profile "))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        DisplayScreenMode::CreatingEditText(buf) => {
            let d = &app.new_display_draft;
            let w = d.canvas_width.parse::<u32>().unwrap_or(1920);
            let h = d.canvas_height.parse::<u32>().unwrap_or(1080);
            let op = d.placard_opacity.parse::<u32>().unwrap_or(90);
            let items = build_display_creating_items(
                &d.wallpaper_color, &d.orientation, w, h,
                &d.placard_color, &d.placard_text_color, op, &d.name, Some(d.current_field), buf);
            let mut state = ListState::default(); state.select(Some(d.current_field));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(" New Display Profile "))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        DisplayScreenMode::CreatingName(buf) => {
            let d = &app.new_display_draft;
            let w = d.canvas_width.parse::<u32>().unwrap_or(1920);
            let h = d.canvas_height.parse::<u32>().unwrap_or(1080);
            let op = d.placard_opacity.parse::<u32>().unwrap_or(90);
            let items = build_display_creating_items(
                &d.wallpaper_color, &d.orientation, w, h,
                &d.placard_color, &d.placard_text_color, op, buf, Some(8), buf);
            let mut state = ListState::default(); state.select(Some(8));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(" New Display Profile "))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }
    }

    let footer_hints: Vec<(&str, &str)> = match &app.display_mode {
        DisplayScreenMode::Browse if app.display_profiles.is_empty() => vec![("a", "add"), ("Esc", "back")],
        DisplayScreenMode::Browse => vec![("↑↓", "select"), ("Enter", "edit"), ("a", "add"), ("d", "delete"), ("Esc", "back")],
        DisplayScreenMode::Detail => vec![("↑↓", "navigate"), ("Enter", "edit"), ("Esc", "back")],
        DisplayScreenMode::EditingText(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
        DisplayScreenMode::CreatingProfile => vec![("↑↓", "navigate"), ("Enter", "select"), ("Esc", "cancel")],
        DisplayScreenMode::CreatingEditText(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
        DisplayScreenMode::CreatingName(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
    };
    render_footer(frame, footer_area, &footer_hints);
}

// ---------------------------------------------------------------------------
// Field item builders
// ---------------------------------------------------------------------------

fn build_taste_detail_items(
    date_start: Option<i64>, date_end: Option<i64>, is_public_domain: bool,
    dept_count: usize, editing_field: Option<usize>, edit_buf: &str,
) -> Vec<ListItem<'static>> {
    let ds = if editing_field == Some(0) { format!("{}▌", edit_buf) } else { date_start.map(|v| v.to_string()).unwrap_or_else(|| "(not set)".to_string()) };
    let de = if editing_field == Some(1) { format!("{}▌", edit_buf) } else { date_end.map(|v| v.to_string()).unwrap_or_else(|| "(not set)".to_string()) };
    let dept_str = if dept_count == 0 { "(any)".to_string() } else { format!("{} selected", dept_count) };
    vec![
        ListItem::new(format!(" {:<16}{}", "Date Start", ds)),
        ListItem::new(format!(" {:<16}{}", "Date End", de)),
        ListItem::new(format!(" {:<16}{}", "Public Domain", if is_public_domain { "Yes" } else { "No" })),
        ListItem::new(format!(" {:<16}{}", "Departments", dept_str)),
    ]
}

fn build_taste_creating_items(
    date_start: Option<i64>, date_end: Option<i64>, is_public_domain: bool,
    dept_count: usize, name: &str, editing_field: Option<usize>, edit_buf: &str,
) -> Vec<ListItem<'static>> {
    let ds = if editing_field == Some(0) { format!("{}▌", edit_buf) } else { date_start.map(|v| v.to_string()).unwrap_or_else(|| "(not set)".to_string()) };
    let de = if editing_field == Some(1) { format!("{}▌", edit_buf) } else { date_end.map(|v| v.to_string()).unwrap_or_else(|| "(not set)".to_string()) };
    let nm = if editing_field == Some(4) { format!("{}▌", edit_buf) } else if name.is_empty() { "(enter name)".to_string() } else { name.to_string() };
    let dept_str = if dept_count == 0 { "(any)".to_string() } else { format!("{} selected", dept_count) };
    vec![
        ListItem::new(format!(" {:<16}{}", "Date Start", ds)),
        ListItem::new(format!(" {:<16}{}", "Date End", de)),
        ListItem::new(format!(" {:<16}{}", "Public Domain", if is_public_domain { "Yes" } else { "No" })),
        ListItem::new(format!(" {:<16}{}", "Departments", dept_str)),
        ListItem::new(format!(" {:<16}{}", "Name", nm)),
    ]
}

fn build_display_detail_items(
    wallpaper_color: &str, orientation: &str, canvas_width: u32, canvas_height: u32,
    placard_color: &str, placard_text_color: &str, placard_opacity: u32,
    editing_field: Option<usize>, edit_buf: &str,
) -> Vec<ListItem<'static>> {
    let color   = if editing_field == Some(0) { format!("{}▌", edit_buf) } else { wallpaper_color.to_string() };
    let orient  = if orientation == "horizontal" { "Horizontal" } else { "Vertical" }.to_string();
    let w_str   = if editing_field == Some(3) { format!("{}▌", edit_buf) } else { canvas_width.to_string() };
    let h_str   = if editing_field == Some(4) { format!("{}▌", edit_buf) } else { canvas_height.to_string() };
    let pc      = if editing_field == Some(5) { format!("{}▌", edit_buf) } else { placard_color.to_string() };
    let ptc     = if editing_field == Some(6) { format!("{}▌", edit_buf) } else { placard_text_color.to_string() };
    let opacity = if editing_field == Some(7) { format!("{}▌", edit_buf) } else { format!("{}%", placard_opacity) };
    vec![
        ListItem::new(format!(" {:<20}{}", "BG Color", color)),
        ListItem::new(format!(" {:<20}{}", "Frame Style", "(coming soon)")).style(Style::default().fg(Color::DarkGray)),
        ListItem::new(format!(" {:<20}{}", "Orientation", orient)),
        ListItem::new(format!(" {:<20}{}", "Width (px)", w_str)),
        ListItem::new(format!(" {:<20}{}", "Height (px)", h_str)),
        ListItem::new(format!(" {:<20}{}", "Placard BG", pc)),
        ListItem::new(format!(" {:<20}{}", "Placard Text", ptc)),
        ListItem::new(format!(" {:<20}{}", "Placard Opacity", opacity)),
    ]
}

fn build_display_creating_items(
    wallpaper_color: &str, orientation: &str, canvas_width: u32, canvas_height: u32,
    placard_color: &str, placard_text_color: &str, placard_opacity: u32,
    name: &str, editing_field: Option<usize>, edit_buf: &str,
) -> Vec<ListItem<'static>> {
    let color   = if editing_field == Some(0) { format!("{}▌", edit_buf) } else { wallpaper_color.to_string() };
    let orient  = if orientation == "horizontal" { "Horizontal" } else { "Vertical" }.to_string();
    let w_str   = if editing_field == Some(3) { format!("{}▌", edit_buf) } else { canvas_width.to_string() };
    let h_str   = if editing_field == Some(4) { format!("{}▌", edit_buf) } else { canvas_height.to_string() };
    let pc      = if editing_field == Some(5) { format!("{}▌", edit_buf) } else { placard_color.to_string() };
    let ptc     = if editing_field == Some(6) { format!("{}▌", edit_buf) } else { placard_text_color.to_string() };
    let opacity = if editing_field == Some(7) { format!("{}▌", edit_buf) } else { format!("{}%", placard_opacity) };
    let nm      = if editing_field == Some(8) { format!("{}▌", edit_buf) } else if name.is_empty() { "(enter name)".to_string() } else { name.to_string() };
    vec![
        ListItem::new(format!(" {:<20}{}", "BG Color", color)),
        ListItem::new(format!(" {:<20}{}", "Frame Style", "(coming soon)")).style(Style::default().fg(Color::DarkGray)),
        ListItem::new(format!(" {:<20}{}", "Orientation", orient)),
        ListItem::new(format!(" {:<20}{}", "Width (px)", w_str)),
        ListItem::new(format!(" {:<20}{}", "Height (px)", h_str)),
        ListItem::new(format!(" {:<20}{}", "Placard BG", pc)),
        ListItem::new(format!(" {:<20}{}", "Placard Text", ptc)),
        ListItem::new(format!(" {:<20}{}", "Placard Opacity", opacity)),
        ListItem::new(format!(" {:<20}{}", "Name", nm)),
    ]
}

fn render_department_picker(frame: &mut Frame, area: Rect, available: &[String], selected: &[String], cursor: usize) {
    if available.is_empty() {
        let msg = Paragraph::new("(collection.db not found — run build_db.py first)")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow)).title(" Select Departments "));
        frame.render_widget(msg, area);
    } else {
        let items: Vec<ListItem> = available.iter().map(|dept| {
            let prefix = if selected.contains(dept) { "[✓] " } else { "[ ] " };
            ListItem::new(format!("{}{}", prefix, dept))
        }).collect();
        let mut state = ListState::default(); state.select(Some(cursor));
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Select Departments (none = all) "))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut state);
    }
}

fn empty_actions_panel<'a>(hints: &[(&'a str, &'a str)]) -> Paragraph<'a> {
    let mut lines = vec![Line::from("")];
    for (key, desc) in hints {
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", key), Style::default().fg(Color::Yellow)),
            Span::raw(desc.to_string()),
        ]));
    }
    Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)).title(" Actions "),
    )
}

// ---------------------------------------------------------------------------
// Build wizard
// ---------------------------------------------------------------------------

fn draw_build(frame: &mut Frame, app: &App) {
    match app.build_step {
        BuildStep::Running => { draw_build_running(frame, app); return; }
        BuildStep::Done    => { draw_build_done(frame, app);    return; }
        _ => {}
    }

    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Build Wallpaper Gallery");

    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    // Step indicator
    let steps = [
        ("1. Taste Profile",   BuildStep::PickTaste),
        ("2. Display Profile", BuildStep::PickDisplay),
        ("3. Output Dir",      BuildStep::PickOutputDir),
        ("4. Count",           BuildStep::PickCount),
    ];
    let mut step_spans: Vec<Span> = Vec::new();
    for (i, (label, step)) in steps.iter().enumerate() {
        if i > 0 { step_spans.push(Span::raw("  →  ")); }
        if *step == app.build_step {
            step_spans.push(Span::styled(*label, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        } else {
            step_spans.push(Span::styled(*label, Style::default().fg(Color::DarkGray)));
        }
    }
    let step_indicator = Paragraph::new(Line::from(step_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(step_indicator, body[0]);

    match app.build_step {
        BuildStep::PickTaste => {
            if app.taste_profiles.is_empty() {
                frame.render_widget(no_profiles_msg("Select Taste Profile"), body[1]);
            } else {
                let items: Vec<ListItem> = app.taste_profiles.iter().map(|p| ListItem::new(p.name.as_str())).collect();
                let mut state = ListState::default(); state.select(Some(app.build_taste_idx));
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray)).title(" Select Taste Profile "))
                    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .highlight_symbol("> ");
                frame.render_stateful_widget(list, body[1], &mut state);
            }
        }

        BuildStep::PickDisplay => {
            if app.display_profiles.is_empty() {
                frame.render_widget(no_profiles_msg("Select Display Profile"), body[1]);
            } else {
                let items: Vec<ListItem> = app.display_profiles.iter().map(|p| ListItem::new(p.name.as_str())).collect();
                let mut state = ListState::default(); state.select(Some(app.build_display_idx));
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray)).title(" Select Display Profile "))
                    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .highlight_symbol("> ");
                frame.render_stateful_widget(list, body[1], &mut state);
            }
        }

        BuildStep::PickOutputDir => {
            let input_text = format!("{}▌", app.build_output_dir);
            let input = Paragraph::new(input_text).block(
                Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)).title(" Output directory "),
            );
            frame.render_widget(input, body[1]);
        }

        BuildStep::PickCount => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Min(0)])
                .split(body[1]);

            let count_text = format!("{}▌", app.build_count_str);
            let input = Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![Span::raw("  Number of wallpapers to generate:")]),
                Line::from(vec![Span::styled(format!("  {}", count_text), Style::default().fg(Color::Yellow))]),
            ])
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow)).title(" Wallpaper Count "));
            frame.render_widget(input, chunks[0]);

            // Summary of what will be built
            let taste_name   = app.taste_profiles.get(app.build_taste_idx).map(|p| p.name.as_str()).unwrap_or("—");
            let display_name = app.display_profiles.get(app.build_display_idx).map(|p| p.name.as_str()).unwrap_or("—");
            let summary = Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![Span::raw(format!("  Taste profile:    {}", taste_name))]),
                Line::from(vec![Span::raw(format!("  Display profile:  {}", display_name))]),
                Line::from(vec![Span::raw(format!("  Output dir:       {}", app.build_output_dir))]),
            ])
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)).title(" Build Summary "));
            frame.render_widget(summary, chunks[1]);
        }

        BuildStep::Running | BuildStep::Done => unreachable!(),
    }

    let footer_hints: &[(&str, &str)] = match app.build_step {
        BuildStep::PickOutputDir => &[("Enter", "next"), ("Esc", "back"), ("Backspace", "edit")],
        BuildStep::PickCount     => &[("Enter", "start build"), ("Esc", "back")],
        _                        => &[("↑↓", "select"), ("Enter", "next"), ("Esc", "back")],
    };
    render_footer(frame, footer_area, footer_hints);
}

fn draw_build_running(frame: &mut Frame, app: &App) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Building…");

    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    // Progress bar
    let (current, total) = app.build_progress;
    let ratio = if total > 0 { current as f64 / total as f64 } else { 0.0 };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" {} ", app.build_phase)))
        .gauge_style(Style::default().fg(Color::Yellow))
        .ratio(ratio.min(1.0))
        .label(format!("{}/{}", current, total));
    frame.render_widget(gauge, body[0]);

    // Log
    let log_lines: Vec<Line> = app.build_log.iter().rev()
        .take(body[1].height.saturating_sub(2) as usize)
        .rev()
        .map(|msg| {
            let color = if msg.contains('✗') || msg.starts_with("ERROR") {
                Color::Red
            } else if msg.contains('✓') {
                Color::Green
            } else {
                Color::Gray
            };
            Line::from(vec![Span::styled(msg.clone(), Style::default().fg(color))])
        })
        .collect();

    let log = Paragraph::new(log_lines)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)).title(" Log "));
    frame.render_widget(log, body[1]);

    render_footer(frame, footer_area, &[("…", "build in progress")]);
}

fn draw_build_done(frame: &mut Frame, app: &App) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Build Complete");

    let body = body_area.inner(Margin { horizontal: 2, vertical: 1 });

    let had_errors = app.build_log.iter().any(|m| m.contains('✗') || m.starts_with("ERROR"));
    let title_color = if had_errors { Color::Yellow } else { Color::Green };

    let summary = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {} wallpapers generated", app.build_produced),
                Style::default().fg(title_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::raw(format!("  {} skipped (download or render errors)", app.build_skipped))]),
        Line::from(""),
        Line::from(vec![Span::raw(format!("  Output: {}", app.build_done_dir))]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Point your wallpaper manager at the output directory.",
            Style::default().fg(Color::DarkGray),
        )]),
    ])
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow)).title(" Done "));
    frame.render_widget(summary, body);

    render_footer(frame, footer_area, &[("Enter", "back to menu")]);
}

// ---------------------------------------------------------------------------
// Small widget helpers
// ---------------------------------------------------------------------------

fn main_item_description<'a>(item: MainItem, cache_size: &'a str) -> Text<'a> {
    match item {
        MainItem::ClearCache => Text::from(Line::from(vec![
            Span::raw("Delete cached artwork images "),
            Span::styled(format!("({})", cache_size), Style::default().fg(Color::Cyan)),
            Span::raw(" to free up disk space. Also clears the URL cache so images will be re-fetched on the next build."),
        ])),
        _ => Text::from(item.description(cache_size)),
    }
}

fn no_profiles_msg(title: &str) -> Paragraph<'_> {
    Paragraph::new("No profiles — create one first.")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" {} ", title)))
}
