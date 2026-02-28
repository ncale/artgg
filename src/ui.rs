use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, BuildStep, MainItem, ProfileMode, Screen, TasteScreenMode};

pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Main => draw_main(frame, app),
        Screen::TasteProfiles => draw_taste_profiles(frame, app),
        Screen::DisplayProfiles => {
            let names: Vec<&str> = app
                .display_profiles
                .iter()
                .map(|p| p.name.as_str())
                .collect();
            draw_profiles(
                frame,
                "Display Profiles",
                &names,
                app.display_selected,
                &app.display_mode,
            );
        }
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

    // Right pane: mode-dependent
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
                let profile = &app.taste_profiles[app.taste_selected];
                let field_items = build_field_items(
                    profile.date_start,
                    profile.date_end,
                    profile.is_public_domain,
                    profile.keywords.len(),
                    None,
                    "",
                );
                let summary = List::new(field_items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(format!(" {} ", profile.name)),
                );
                frame.render_widget(summary, body[1]);
            }
        }
        TasteScreenMode::Detail => {
            let profile = &app.taste_profiles[app.taste_selected];
            let field_items = build_field_items(
                profile.date_start,
                profile.date_end,
                profile.is_public_domain,
                profile.keywords.len(),
                None,
                "",
            );
            let mut detail_state = ListState::default();
            detail_state.select(Some(app.taste_detail_field));
            let detail = List::new(field_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(format!(" {} ", profile.name)),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(detail, body[1], &mut detail_state);
        }
        TasteScreenMode::EditingDate(buf) => {
            let profile = &app.taste_profiles[app.taste_selected];
            let field_items = build_field_items(
                profile.date_start,
                profile.date_end,
                profile.is_public_domain,
                profile.keywords.len(),
                Some(app.taste_detail_field),
                buf,
            );
            let mut detail_state = ListState::default();
            detail_state.select(Some(app.taste_detail_field));
            let detail = List::new(field_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(format!(" {} ", profile.name)),
                )
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(detail, body[1], &mut detail_state);
        }
        TasteScreenMode::SelectingKeywords => {
            let profile = &app.taste_profiles[app.taste_selected];
            if app.available_keywords.is_empty() {
                let msg = Paragraph::new("(no keywords in database yet)")
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Yellow))
                            .title(" Select Keywords "),
                    );
                frame.render_widget(msg, body[1]);
            } else {
                let kw_items: Vec<ListItem> = app
                    .available_keywords
                    .iter()
                    .map(|(_, kw)| {
                        let checked = profile.keywords.contains(kw);
                        let prefix = if checked { "[✓] " } else { "[ ] " };
                        ListItem::new(format!("{}{}", prefix, kw))
                    })
                    .collect();
                let mut kw_state = ListState::default();
                kw_state.select(Some(app.keyword_cursor));
                let kw_list = List::new(kw_items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Yellow))
                            .title(" Select Keywords "),
                    )
                    .highlight_style(
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");
                frame.render_stateful_widget(kw_list, body[1], &mut kw_state);
            }
        }
        TasteScreenMode::Adding(buf) => {
            let input_text = format!("{}▌", buf);
            let input = Paragraph::new(input_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" New profile name "),
            );
            frame.render_widget(input, body[1]);
        }
    }

    // Footer hints
    let kw_count = if !app.taste_profiles.is_empty() {
        app.taste_profiles[app.taste_selected].keywords.len()
    } else {
        0
    };
    let toggle_hint = format!("toggle ({}/10)", kw_count);
    let footer_hints: Vec<(&str, &str)> = match &app.taste_mode {
        TasteScreenMode::Browse if app.taste_profiles.is_empty() => {
            vec![("a", "add"), ("Esc", "back")]
        }
        TasteScreenMode::Browse => vec![
            ("↑↓", "select"),
            ("Enter", "edit"),
            ("a", "add"),
            ("d", "delete"),
            ("Esc", "back"),
        ],
        TasteScreenMode::Detail => {
            vec![("↑↓", "navigate"), ("Enter", "edit"), ("Esc", "back")]
        }
        TasteScreenMode::EditingDate(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
        TasteScreenMode::SelectingKeywords => vec![
            ("↑↓", "navigate"),
            ("Space", toggle_hint.as_str()),
            ("Esc", "done"),
        ],
        TasteScreenMode::Adding(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
    };
    render_footer(frame, footer_area, &footer_hints);
}

/// Builds the 5 field rows for the taste profile detail pane.
/// `editing_field`: Some(idx) if a date field is being edited, None otherwise.
/// `edit_buf`: the current editing buffer (used when editing_field is Some).
fn build_field_items(
    date_start: Option<i64>,
    date_end: Option<i64>,
    is_public_domain: bool,
    kw_count: usize,
    editing_field: Option<usize>,
    edit_buf: &str,
) -> Vec<ListItem<'static>> {
    let date_start_val = if editing_field == Some(0) {
        format!("{}▌", edit_buf)
    } else {
        date_start
            .map(|v| v.to_string())
            .unwrap_or_else(|| "(not set)".to_string())
    };
    let date_end_val = if editing_field == Some(1) {
        format!("{}▌", edit_buf)
    } else {
        date_end
            .map(|v| v.to_string())
            .unwrap_or_else(|| "(not set)".to_string())
    };
    let pd_val = if is_public_domain { "Yes" } else { "No" }.to_string();
    let kw_val = format!("{}/10", kw_count);

    vec![
        ListItem::new(format!(" {:<16}{}", "Date Start", date_start_val)),
        ListItem::new(format!(" {:<16}{}", "Date End", date_end_val)),
        ListItem::new(format!(" {:<16}{}", "Public Domain", pd_val)),
        ListItem::new(format!(" {:<16}{}", "Keywords", kw_val)),
        ListItem::new(format!(" {:<16}{}", "Artists", "(coming soon)"))
            .style(Style::default().fg(Color::DarkGray)),
    ]
}

fn draw_profiles(
    frame: &mut Frame,
    title: &str,
    names: &[&str],
    selected: usize,
    mode: &ProfileMode,
) {
    let (header_area, body_area, footer_area) = base_layout(frame);
    render_header(frame, header_area, title);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(body_area.inner(Margin { horizontal: 2, vertical: 1 }));

    let items: Vec<ListItem> = if names.is_empty() {
        vec![ListItem::new("(none)").style(Style::default().fg(Color::DarkGray))]
    } else {
        names.iter().map(|n| ListItem::new(*n)).collect()
    };

    let mut list_state = ListState::default();
    if !names.is_empty() {
        list_state.select(Some(selected));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" {} ", title)),
        )
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, body[0], &mut list_state);

    match mode {
        ProfileMode::Browse => {
            let info = Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(" a ", Style::default().fg(Color::Yellow)),
                    Span::raw("add profile"),
                ]),
                Line::from(vec![
                    Span::styled(" d ", Style::default().fg(Color::Yellow)),
                    Span::raw("delete selected"),
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
        }
        ProfileMode::Adding(buf) => {
            let input_text = format!("{}▌", buf);
            let input = Paragraph::new(input_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" New profile name "),
            );
            frame.render_widget(input, body[1]);
        }
    }

    let footer_hints: &[(&str, &str)] = match mode {
        ProfileMode::Browse => &[("a", "add"), ("d", "delete"), ("↑↓", "select"), ("Esc", "back")],
        ProfileMode::Adding(_) => &[("Enter", "confirm"), ("Esc", "cancel")],
    };
    render_footer(frame, footer_area, footer_hints);
}

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
