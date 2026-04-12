use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Frame,
};

use super::{App, FormField, FormKind, Mode};

pub fn draw(f: &mut Frame, app: &App) {
    let is_searching = app.mode == Mode::Search;

    let mut constraints = vec![
        Constraint::Length(3), // title
    ];
    if is_searching || !app.search_query.is_empty() {
        constraints.push(Constraint::Length(3)); // search bar
    }
    constraints.push(Constraint::Min(5)); // table
    constraints.push(Constraint::Length(3)); // action bar / status

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut chunk_idx = 0;

    // -- Title bar --
    draw_title(f, app, chunks[chunk_idx]);
    chunk_idx += 1;

    // -- Search bar (visible when searching or query active) --
    if is_searching || !app.search_query.is_empty() {
        draw_search_bar(f, app, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // -- App table --
    draw_table(f, app, chunks[chunk_idx]);
    chunk_idx += 1;

    // -- Action bar / Status --
    draw_action_bar(f, app, chunks[chunk_idx]);

    // -- Popup overlays --
    match &app.mode {
        Mode::Form(kind) => draw_form_popup(f, app, kind),
        Mode::ConfirmDelete => draw_confirm_popup(f, app),
        _ => {}
    }
}

fn draw_title(f: &mut Frame, app: &App, area: Rect) {
    let count = app.apps.len();
    let count_str = if count == 1 {
        "1 app".to_string()
    } else {
        format!("{} apps", count)
    };

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " Lycan ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("— PWA Manager", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled(
            format!("[{}]", count_str),
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.mode == Mode::Search;

    let style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let search_text = format!(" / {}", app.search_query);
    let display = if is_active {
        format!("{}▌", search_text)
    } else {
        search_text
    };

    let matched = app.filtered_indices.len();
    let total = app.apps.len();
    let count_hint = if app.search_query.is_empty() {
        String::new()
    } else {
        format!(" ({}/{})", matched, total)
    };

    let search = Paragraph::new(Line::from(vec![
        Span::styled(display, style),
        Span::styled(count_hint, Style::default().fg(Color::DarkGray)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Search "),
    );
    f.render_widget(search, area);
}

fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    if app.filtered_indices.is_empty() {
        let msg = if app.apps.is_empty() {
            Line::from(vec![
                Span::styled("No PWAs yet. Press ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "A",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to add one.", Style::default().fg(Color::DarkGray)),
            ])
        } else {
            Line::from(Span::styled(
                "No matches found.",
                Style::default().fg(Color::DarkGray),
            ))
        };
        let empty = Paragraph::new(msg)
            .block(Block::default().borders(Borders::ALL).title(" Apps "));
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("  "),
        Cell::from("Name").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("URL").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Created").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &real_idx)| {
            let a = &app.apps[real_idx];
            let is_selected = display_idx == app.selected;

            let marker = if is_selected { ">>" } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let date = a.created_at.format("%Y-%m-%d").to_string();

            Row::new(vec![
                Cell::from(marker).style(if is_selected {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                }),
                Cell::from(a.name.clone()),
                Cell::from(truncate_url(&a.url)),
                Cell::from(date),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Apps "));

    f.render_widget(table, area);
}

fn draw_action_bar(f: &mut Frame, app: &App, area: Rect) {
    if let Some((ref msg, is_error)) = app.status_message {
        let color = if is_error { Color::Red } else { Color::Green };
        let status = Paragraph::new(Span::styled(
            format!(" {}", msg),
            Style::default().fg(color),
        ))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, area);
        return;
    }

    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Color::DarkGray);
    let sep = Span::styled("   ", label_style);

    let actions = vec![
        Span::raw(" "),
        Span::styled("A", key_style),
        Span::styled("dd", label_style),
        sep.clone(),
        Span::styled("E", key_style),
        Span::styled("dit", label_style),
        sep.clone(),
        Span::styled("O", key_style),
        Span::styled("pen", label_style),
        sep.clone(),
        Span::styled("D", key_style),
        Span::styled("elete", label_style),
        Span::styled("          ", label_style),
        Span::styled("/", key_style),
        Span::styled(" Search", label_style),
        sep,
        Span::styled("Q", key_style),
        Span::styled("uit", label_style),
    ];

    let bar = Paragraph::new(Line::from(actions)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(bar, area);
}

fn draw_form_popup(f: &mut Frame, app: &App, kind: &FormKind) {
    let title = match kind {
        FormKind::Add => " Add PWA ",
        FormKind::Edit(_) => " Edit PWA ",
    };

    let popup_area = centered_rect(50, 12, f.area());
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    // Name field
    let name_active = app.form_field == FormField::Name;
    let name_border = if name_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let name_text = if name_active {
        format!("{}▌", app.name_input)
    } else {
        app.name_input.clone()
    };
    let name_widget = Paragraph::new(name_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(name_border)
                .title(" App Name "),
        );
    f.render_widget(name_widget, form_chunks[0]);

    // URL field
    let url_active = app.form_field == FormField::Url;
    let url_border = if url_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let url_text = if url_active {
        format!("{}▌", app.url_input)
    } else {
        app.url_input.clone()
    };
    let url_widget = Paragraph::new(url_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(url_border)
                .title(" URL "),
        );
    f.render_widget(url_widget, form_chunks[1]);

    // Hints
    let hints = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::Yellow)),
        Span::styled(": switch  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Ctrl+S", Style::default().fg(Color::Yellow)),
        Span::styled(": save  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled(": cancel", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(hints, form_chunks[2]);
}

fn draw_confirm_popup(f: &mut Frame, app: &App) {
    let name = app
        .selected_app()
        .map(|a| a.name.as_str())
        .unwrap_or("?");

    let popup_area = centered_rect(40, 7, f.area());
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm Delete ");

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(inner);

    let msg = Paragraph::new(Line::from(vec![
        Span::raw(" Delete "),
        Span::styled(
            name,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("?"),
    ]));
    f.render_widget(msg, chunks[0]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled(" Y", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("es  ", Style::default().fg(Color::DarkGray)),
        Span::styled("N", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled("o (any key)", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(hint, chunks[1]);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn truncate_url(url: &str) -> String {
    url.replace("https://", "")
        .replace("http://", "")
        .trim_end_matches('/')
        .to_string()
}
