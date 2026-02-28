use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, BuildStep, MainItem, ProfileMode, Screen};

pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Main => draw_main(frame, app),
        Screen::TasteProfiles => {
            let names: Vec<&str> = app.taste_profiles.iter().map(|p| p.name.as_str()).collect();
            draw_profiles(frame, "Taste Profiles", &names, app.taste_selected, &app.taste_mode);
        }
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
