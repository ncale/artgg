use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, BuildStep, DisplayScreenMode, MainItem, Screen, TasteScreenMode};

pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Main => draw_main(frame, app),
        Screen::TasteProfiles => draw_taste_profiles(frame, app),
        Screen::DisplayProfiles => draw_display_profiles(frame, app),
        Screen::Build => draw_build(frame, app),
    }
}

fn base_layout(frame: &Frame) -> (Rect, Rect, Rect) {
    let area = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}

fn render_header(frame: &mut Frame, area: Rect, subtitle: &str) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "art",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "gg",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
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
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default().fg(Color::Yellow),
        ));
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

fn draw_main(frame: &mut Frame, app: &App) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Classical artwork wallpaper generator");

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(0)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    let items: Vec<ListItem> = MainItem::ALL
        .iter()
        .map(|item| {
            if item.is_disabled() {
                ListItem::new(item.label()).style(Style::default().fg(Color::DarkGray))
            } else {
                ListItem::new(item.label())
            }
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.main_selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Menu "),
        )
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, body[0], &mut list_state);

    let selected_item = MainItem::ALL[app.main_selected];
    let detail = Paragraph::new(selected_item.description())
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" {} ", selected_item.label())),
        );
    frame.render_widget(detail, body[1]);

    render_footer(
        frame,
        footer_area,
        &[("↑↓", "navigate"), ("Enter", "select"), ("q", "quit")],
    );
}

// ─── Taste Profiles ───────────────────────────────────────────────────────────

fn draw_taste_profiles(frame: &mut Frame, app: &App) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Taste Profiles");

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    // Left pane: profile list
    let left_items: Vec<ListItem> = if app.taste_profiles.is_empty() {
        vec![ListItem::new("(none)").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.taste_profiles
            .iter()
            .map(|p| ListItem::new(p.name.as_str()))
            .collect()
    };
    let mut left_state = ListState::default();
    if !app.taste_profiles.is_empty() {
        left_state.select(Some(app.taste_selected));
    }
    let left_list = List::new(left_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Taste Profiles "),
        )
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    frame.render_stateful_widget(left_list, body[0], &mut left_state);

    // Right pane
    match &app.taste_mode {
        TasteScreenMode::Browse => {
            if app.taste_profiles.is_empty() {
                let info = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(" a ", Style::default().fg(Color::Yellow)),
                        Span::raw("add profile"),
                    ]),
                    Line::from(vec![
                        Span::styled(" Esc ", Style::default().fg(Color::Yellow)),
                        Span::raw("back to menu"),
                    ]),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Actions "),
                );
                frame.render_widget(info, body[1]);
            } else {
                let p = &app.taste_profiles[app.taste_selected];
                let items = build_taste_detail_items(
                    p.date_start, p.date_end, p.is_public_domain, p.keywords.len(),
                    None, "",
                );
                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(format!(" {} ", p.name)),
                );
                frame.render_widget(list, body[1]);
            }
        }

        TasteScreenMode::Detail => {
            let p = &app.taste_profiles[app.taste_selected];
            let items = build_taste_detail_items(
                p.date_start, p.date_end, p.is_public_domain, p.keywords.len(),
                None, "",
            );
            let mut state = ListState::default();
            state.select(Some(app.taste_detail_field));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(format!(" {} ", p.name)),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        TasteScreenMode::EditingDate(buf) => {
            let p = &app.taste_profiles[app.taste_selected];
            let items = build_taste_detail_items(
                p.date_start, p.date_end, p.is_public_domain, p.keywords.len(),
                Some(app.taste_detail_field), buf,
            );
            let mut state = ListState::default();
            state.select(Some(app.taste_detail_field));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(format!(" {} ", p.name)),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        TasteScreenMode::SelectingKeywords => {
            let p = &app.taste_profiles[app.taste_selected];
            render_keyword_picker(frame, body[1], &app.available_keywords, &p.keywords, app.keyword_cursor);
        }

        TasteScreenMode::CreatingProfile => {
            let d = &app.new_taste_draft;
            let items = build_taste_creating_items(
                d.date_start, d.date_end, d.is_public_domain, d.keywords.len(),
                &d.name, None, "",
            );
            let mut state = ListState::default();
            state.select(Some(d.current_field));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(" New Taste Profile "),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        TasteScreenMode::CreatingEditDate(buf) => {
            let d = &app.new_taste_draft;
            let items = build_taste_creating_items(
                d.date_start, d.date_end, d.is_public_domain, d.keywords.len(),
                &d.name, Some(d.current_field), buf,
            );
            let mut state = ListState::default();
            state.select(Some(d.current_field));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(" New Taste Profile "),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        TasteScreenMode::CreatingSelectKeywords => {
            render_keyword_picker(
                frame, body[1],
                &app.available_keywords,
                &app.new_taste_draft.keywords,
                app.keyword_cursor,
            );
        }

        TasteScreenMode::CreatingName(buf) => {
            let d = &app.new_taste_draft;
            let items = build_taste_creating_items(
                d.date_start, d.date_end, d.is_public_domain, d.keywords.len(),
                buf, Some(4), buf,
            );
            let mut state = ListState::default();
            state.select(Some(4));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(" New Taste Profile "),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }
    }

    // Footer
    let kw_count = match &app.taste_mode {
        TasteScreenMode::SelectingKeywords => {
            if !app.taste_profiles.is_empty() {
                app.taste_profiles[app.taste_selected].keywords.len()
            } else {
                0
            }
        }
        TasteScreenMode::CreatingSelectKeywords => app.new_taste_draft.keywords.len(),
        _ => 0,
    };
    let toggle_hint = format!("toggle ({}/10)", kw_count);
    let footer_hints: Vec<(&str, &str)> = match &app.taste_mode {
        TasteScreenMode::Browse if app.taste_profiles.is_empty() => {
            vec![("a", "add"), ("Esc", "back")]
        }
        TasteScreenMode::Browse => vec![
            ("↑↓", "select"), ("Enter", "edit"), ("a", "add"), ("d", "delete"), ("Esc", "back"),
        ],
        TasteScreenMode::Detail => vec![("↑↓", "navigate"), ("Enter", "edit"), ("Esc", "back")],
        TasteScreenMode::EditingDate(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
        TasteScreenMode::SelectingKeywords => vec![
            ("↑↓", "navigate"), ("Space", toggle_hint.as_str()), ("Esc", "done"),
        ],
        TasteScreenMode::CreatingProfile => {
            vec![("↑↓", "navigate"), ("Enter", "select"), ("Esc", "cancel")]
        }
        TasteScreenMode::CreatingEditDate(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
        TasteScreenMode::CreatingSelectKeywords => vec![
            ("↑↓", "navigate"), ("Space", toggle_hint.as_str()), ("Esc", "done"),
        ],
        TasteScreenMode::CreatingName(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
    };
    render_footer(frame, footer_area, &footer_hints);
}

// ─── Display Profiles ─────────────────────────────────────────────────────────

fn draw_display_profiles(frame: &mut Frame, app: &App) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Display Profiles");

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    // Left pane
    let left_items: Vec<ListItem> = if app.display_profiles.is_empty() {
        vec![ListItem::new("(none)").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.display_profiles
            .iter()
            .map(|p| ListItem::new(p.name.as_str()))
            .collect()
    };
    let mut left_state = ListState::default();
    if !app.display_profiles.is_empty() {
        left_state.select(Some(app.display_selected));
    }
    let left_list = List::new(left_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Display Profiles "),
        )
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    frame.render_stateful_widget(left_list, body[0], &mut left_state);

    // Right pane
    match &app.display_mode {
        DisplayScreenMode::Browse => {
            if app.display_profiles.is_empty() {
                let info = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(" a ", Style::default().fg(Color::Yellow)),
                        Span::raw("add profile"),
                    ]),
                    Line::from(vec![
                        Span::styled(" Esc ", Style::default().fg(Color::Yellow)),
                        Span::raw("back to menu"),
                    ]),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Actions "),
                );
                frame.render_widget(info, body[1]);
            } else {
                let p = &app.display_profiles[app.display_selected];
                let items = build_display_detail_items(
                    &p.wallpaper_color, &p.orientation, &p.aspect_ratio,
                    None, "",
                );
                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(format!(" {} ", p.name)),
                );
                frame.render_widget(list, body[1]);
            }
        }

        DisplayScreenMode::Detail => {
            let p = &app.display_profiles[app.display_selected];
            let items = build_display_detail_items(
                &p.wallpaper_color, &p.orientation, &p.aspect_ratio,
                None, "",
            );
            let mut state = ListState::default();
            state.select(Some(app.display_detail_field));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(format!(" {} ", p.name)),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        DisplayScreenMode::EditingText(buf) => {
            let p = &app.display_profiles[app.display_selected];
            let items = build_display_detail_items(
                &p.wallpaper_color, &p.orientation, &p.aspect_ratio,
                Some(app.display_detail_field), buf,
            );
            let mut state = ListState::default();
            state.select(Some(app.display_detail_field));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(format!(" {} ", p.name)),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        DisplayScreenMode::CreatingProfile => {
            let d = &app.new_display_draft;
            let items = build_display_creating_items(
                &d.wallpaper_color, &d.orientation, &d.aspect_ratio,
                &d.name, None, "",
            );
            let mut state = ListState::default();
            state.select(Some(d.current_field));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(" New Display Profile "),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        DisplayScreenMode::CreatingEditText(buf) => {
            let d = &app.new_display_draft;
            let items = build_display_creating_items(
                &d.wallpaper_color, &d.orientation, &d.aspect_ratio,
                &d.name, Some(d.current_field), buf,
            );
            let mut state = ListState::default();
            state.select(Some(d.current_field));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(" New Display Profile "),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }

        DisplayScreenMode::CreatingName(buf) => {
            let d = &app.new_display_draft;
            let items = build_display_creating_items(
                &d.wallpaper_color, &d.orientation, &d.aspect_ratio,
                buf, Some(4), buf,
            );
            let mut state = ListState::default();
            state.select(Some(4));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(" New Display Profile "),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, body[1], &mut state);
        }
    }

    // Footer
    let footer_hints: Vec<(&str, &str)> = match &app.display_mode {
        DisplayScreenMode::Browse if app.display_profiles.is_empty() => {
            vec![("a", "add"), ("Esc", "back")]
        }
        DisplayScreenMode::Browse => vec![
            ("↑↓", "select"), ("Enter", "edit"), ("a", "add"), ("d", "delete"), ("Esc", "back"),
        ],
        DisplayScreenMode::Detail => {
            vec![("↑↓", "navigate"), ("Enter", "edit"), ("Esc", "back")]
        }
        DisplayScreenMode::EditingText(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
        DisplayScreenMode::CreatingProfile => {
            vec![("↑↓", "navigate"), ("Enter", "select"), ("Esc", "cancel")]
        }
        DisplayScreenMode::CreatingEditText(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
        DisplayScreenMode::CreatingName(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
    };
    render_footer(frame, footer_area, &footer_hints);
}

// ─── Field item builders ──────────────────────────────────────────────────────

/// Taste profile detail view: 5 rows (Date Start, Date End, Public Domain, Keywords, Artists).
fn build_taste_detail_items(
    date_start: Option<i64>,
    date_end: Option<i64>,
    is_public_domain: bool,
    kw_count: usize,
    editing_field: Option<usize>,
    edit_buf: &str,
) -> Vec<ListItem<'static>> {
    let ds = if editing_field == Some(0) {
        format!("{}▌", edit_buf)
    } else {
        date_start.map(|v| v.to_string()).unwrap_or_else(|| "(not set)".to_string())
    };
    let de = if editing_field == Some(1) {
        format!("{}▌", edit_buf)
    } else {
        date_end.map(|v| v.to_string()).unwrap_or_else(|| "(not set)".to_string())
    };
    let pd = if is_public_domain { "Yes" } else { "No" }.to_string();
    let kw = format!("{}/10", kw_count);

    vec![
        ListItem::new(format!(" {:<16}{}", "Date Start", ds)),
        ListItem::new(format!(" {:<16}{}", "Date End", de)),
        ListItem::new(format!(" {:<16}{}", "Public Domain", pd)),
        ListItem::new(format!(" {:<16}{}", "Keywords", kw)),
        ListItem::new(format!(" {:<16}{}", "Artists", "(coming soon)"))
            .style(Style::default().fg(Color::DarkGray)),
    ]
}

/// Taste profile creation form: 5 rows (same as detail but Name replaces Artists).
fn build_taste_creating_items(
    date_start: Option<i64>,
    date_end: Option<i64>,
    is_public_domain: bool,
    kw_count: usize,
    name: &str,
    editing_field: Option<usize>,
    edit_buf: &str,
) -> Vec<ListItem<'static>> {
    let ds = if editing_field == Some(0) {
        format!("{}▌", edit_buf)
    } else {
        date_start.map(|v| v.to_string()).unwrap_or_else(|| "(not set)".to_string())
    };
    let de = if editing_field == Some(1) {
        format!("{}▌", edit_buf)
    } else {
        date_end.map(|v| v.to_string()).unwrap_or_else(|| "(not set)".to_string())
    };
    let pd = if is_public_domain { "Yes" } else { "No" }.to_string();
    let kw = format!("{}/10", kw_count);
    let nm = if editing_field == Some(4) {
        format!("{}▌", edit_buf)
    } else if name.is_empty() {
        "(enter name)".to_string()
    } else {
        name.to_string()
    };

    vec![
        ListItem::new(format!(" {:<16}{}", "Date Start", ds)),
        ListItem::new(format!(" {:<16}{}", "Date End", de)),
        ListItem::new(format!(" {:<16}{}", "Public Domain", pd)),
        ListItem::new(format!(" {:<16}{}", "Keywords", kw)),
        ListItem::new(format!(" {:<16}{}", "Name", nm)),
    ]
}

/// Display profile detail view: 4 rows (Color, Frame Style, Orientation, Aspect Ratio).
fn build_display_detail_items(
    wallpaper_color: &str,
    orientation: &str,
    aspect_ratio: &str,
    editing_field: Option<usize>,
    edit_buf: &str,
) -> Vec<ListItem<'static>> {
    let color = if editing_field == Some(0) {
        format!("{}▌", edit_buf)
    } else {
        wallpaper_color.to_string()
    };
    let orient = if orientation == "horizontal" { "Horizontal" } else { "Vertical" }.to_string();
    let ratio = if editing_field == Some(3) {
        format!("{}▌", edit_buf)
    } else {
        aspect_ratio.to_string()
    };

    vec![
        ListItem::new(format!(" {:<16}{}", "Color", color)),
        ListItem::new(format!(" {:<16}{}", "Frame Style", "(coming soon)"))
            .style(Style::default().fg(Color::DarkGray)),
        ListItem::new(format!(" {:<16}{}", "Orientation", orient)),
        ListItem::new(format!(" {:<16}{}", "Aspect Ratio", ratio)),
    ]
}

/// Display profile creation form: 5 rows (same as detail + Name at bottom).
fn build_display_creating_items(
    wallpaper_color: &str,
    orientation: &str,
    aspect_ratio: &str,
    name: &str,
    editing_field: Option<usize>,
    edit_buf: &str,
) -> Vec<ListItem<'static>> {
    let color = if editing_field == Some(0) {
        format!("{}▌", edit_buf)
    } else {
        wallpaper_color.to_string()
    };
    let orient = if orientation == "horizontal" { "Horizontal" } else { "Vertical" }.to_string();
    let ratio = if editing_field == Some(3) {
        format!("{}▌", edit_buf)
    } else {
        aspect_ratio.to_string()
    };
    let nm = if editing_field == Some(4) {
        format!("{}▌", edit_buf)
    } else if name.is_empty() {
        "(enter name)".to_string()
    } else {
        name.to_string()
    };

    vec![
        ListItem::new(format!(" {:<16}{}", "Color", color)),
        ListItem::new(format!(" {:<16}{}", "Frame Style", "(coming soon)"))
            .style(Style::default().fg(Color::DarkGray)),
        ListItem::new(format!(" {:<16}{}", "Orientation", orient)),
        ListItem::new(format!(" {:<16}{}", "Aspect Ratio", ratio)),
        ListItem::new(format!(" {:<16}{}", "Name", nm)),
    ]
}

/// Shared keyword picker used by both SelectingKeywords and CreatingSelectKeywords.
fn render_keyword_picker(
    frame: &mut Frame,
    area: Rect,
    available: &[(i64, String)],
    selected: &[String],
    cursor: usize,
) {
    if available.is_empty() {
        let msg = Paragraph::new("(no keywords in database yet)")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Select Keywords "),
            );
        frame.render_widget(msg, area);
    } else {
        let items: Vec<ListItem> = available
            .iter()
            .map(|(_, kw)| {
                let prefix = if selected.contains(kw) { "[✓] " } else { "[ ] " };
                ListItem::new(format!("{}{}", prefix, kw))
            })
            .collect();
        let mut state = ListState::default();
        state.select(Some(cursor));
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Select Keywords "),
            )
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut state);
    }
}

// ─── Build wizard ─────────────────────────────────────────────────────────────

fn draw_build(frame: &mut Frame, app: &App) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, "Build Wallpaper Gallery");

    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    let steps = [
        ("1. Taste Profile", BuildStep::PickTaste),
        ("2. Display Profile", BuildStep::PickDisplay),
        ("3. Output Dir", BuildStep::PickOutputDir),
    ];
    let mut step_spans: Vec<Span> = Vec::new();
    for (i, (label, step)) in steps.iter().enumerate() {
        if i > 0 {
            step_spans.push(Span::raw("  →  "));
        }
        if *step == app.build_step {
            step_spans.push(Span::styled(
                *label,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
        } else {
            step_spans.push(Span::styled(*label, Style::default().fg(Color::DarkGray)));
        }
    }

    let step_indicator = Paragraph::new(Line::from(step_spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(step_indicator, body[0]);

    match app.build_step {
        BuildStep::PickTaste => {
            if app.taste_profiles.is_empty() {
                let msg = Paragraph::new("No profiles — create one first.")
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::DarkGray))
                            .title(" Select Taste Profile "),
                    );
                frame.render_widget(msg, body[1]);
            } else {
                let items: Vec<ListItem> = app
                    .taste_profiles
                    .iter()
                    .map(|p| ListItem::new(p.name.as_str()))
                    .collect();
                let mut state = ListState::default();
                state.select(Some(app.build_taste_idx));
                let list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::DarkGray))
                            .title(" Select Taste Profile "),
                    )
                    .highlight_style(
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");
                frame.render_stateful_widget(list, body[1], &mut state);
            }
        }
        BuildStep::PickDisplay => {
            if app.display_profiles.is_empty() {
                let msg = Paragraph::new("No profiles — create one first.")
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::DarkGray))
                            .title(" Select Display Profile "),
                    );
                frame.render_widget(msg, body[1]);
            } else {
                let items: Vec<ListItem> = app
                    .display_profiles
                    .iter()
                    .map(|p| ListItem::new(p.name.as_str()))
                    .collect();
                let mut state = ListState::default();
                state.select(Some(app.build_display_idx));
                let list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::DarkGray))
                            .title(" Select Display Profile "),
                    )
                    .highlight_style(
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");
                frame.render_stateful_widget(list, body[1], &mut state);
            }
        }
        BuildStep::PickOutputDir => {
            let input_text = format!("{}▌", app.build_output_dir);
            let input = Paragraph::new(input_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Output directory "),
            );
            frame.render_widget(input, body[1]);
        }
    }

    let footer_hints: &[(&str, &str)] = match app.build_step {
        BuildStep::PickOutputDir => &[("Enter", "build"), ("Esc", "back"), ("Backspace", "edit")],
        _ => &[("↑↓", "select"), ("Enter", "next"), ("Esc", "back")],
    };
    render_footer(frame, footer_area, footer_hints);
}
