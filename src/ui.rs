use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, MenuItem};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.size();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(0),     // body
            Constraint::Length(3),  // footer
        ])
        .split(area);

    // ── Header ──────────────────────────────────────────────────────────────
    let header = Paragraph::new(Line::from(vec![
        Span::styled("art", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("gg", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("  ·  Classical artwork wallpaper generator"),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, outer[0]);

    // ── Body ─────────────────────────────────────────────────────────────────
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(0)])
        .split(outer[1].inner(Margin { horizontal: 2, vertical: 1 }));

    // Menu list
    let items: Vec<ListItem> = MenuItem::ALL
        .iter()
        .map(|item| ListItem::new(item.label()))
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Menu "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, body[0], &mut list_state);

    // Description / status panel
    let selected = app.selected_item();
    let detail_text = if let Some(ref msg) = app.status {
        msg.clone()
    } else {
        selected.description().to_string()
    };

    let detail = Paragraph::new(detail_text)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" {} ", selected.label())),
        );
    frame.render_widget(detail, body[1]);

    // ── Footer ───────────────────────────────────────────────────────────────
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(Color::Yellow)),
        Span::raw("navigate   "),
        Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("select   "),
        Span::styled(" q ", Style::default().fg(Color::Yellow)),
        Span::raw("quit"),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(footer, outer[2]);
}
