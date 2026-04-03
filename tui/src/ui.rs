use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Tabs},
};

use crate::app::{App, Tab};
use crate::client::{TaskStatus, WorkerStatus};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    render_tabs(f, app, chunks[0]);

    match app.active_tab {
        Tab::Queue => render_queue(f, app, chunks[1]),
        Tab::Workers => render_workers(f, app, chunks[1]),
        Tab::Daemon => render_daemon(f, app, chunks[1]),
    }

    render_statusbar(f, app, chunks[2]);
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| {
            let title = t.title();
            if *t == app.active_tab {
                Line::from(Span::styled(title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            } else {
                Line::from(title)
            }
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" team "))
        .select(Tab::ALL.iter().position(|t| *t == app.active_tab).unwrap_or(0))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    f.render_widget(tabs, area);
}

fn render_queue(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec!["ID", "Issue", "Agent", "Status", "Priority"])
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .height(1);

    let rows: Vec<Row> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let status_str = match TaskStatus::try_from(task.status).unwrap_or(TaskStatus::Queued) {
                TaskStatus::Queued => "queued",
                TaskStatus::Running => "running",
                TaskStatus::Completed => "completed",
                TaskStatus::Failed => "failed",
            };
            let status_color = match TaskStatus::try_from(task.status).unwrap_or(TaskStatus::Queued) {
                TaskStatus::Queued => Color::White,
                TaskStatus::Running => Color::Green,
                TaskStatus::Completed => Color::Blue,
                TaskStatus::Failed => Color::Red,
            };

            let issue = format_issue_ref(&task.issue_ref);
            let short_id = task.id.chars().take(8).collect::<String>();

            let style = if i == app.selected_task {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(short_id),
                Cell::from(issue),
                Cell::from(task.agent.as_str()),
                Cell::from(Span::styled(status_str, Style::default().fg(status_color))),
                Cell::from(task.priority.to_string()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Min(20),
        Constraint::Min(15),
        Constraint::Length(10),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Queue ({} tasks) ", app.tasks.len())),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));

    let mut state = TableState::default();
    if !app.tasks.is_empty() {
        state.select(Some(app.selected_task));
    }

    f.render_stateful_widget(table, area, &mut state);
}

fn render_workers(f: &mut Frame, app: &App, area: Rect) {
    let Some(ws) = &app.worker_status else {
        let p = Paragraph::new("No worker data available")
            .block(Block::default().borders(Borders::ALL).title(" Workers "));
        f.render_widget(p, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let summary = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Total: "),
            Span::styled(ws.total.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw("   Busy: "),
            Span::styled(ws.busy.to_string(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("   Idle: "),
            Span::styled(ws.idle.to_string(), Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Summary "));
    f.render_widget(summary, chunks[0]);

    let header = Row::new(vec!["Worker ID", "Status", "Current Task", "Agent"])
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = ws
        .workers
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let (status_str, status_color) =
                match WorkerStatus::try_from(w.status).unwrap_or(WorkerStatus::Idle) {
                    WorkerStatus::Idle => ("idle", Color::Gray),
                    WorkerStatus::Busy => ("busy", Color::Green),
                };

            let short_id = w.worker_id.chars().take(8).collect::<String>();
            let task_id = if w.current_task_id.is_empty() {
                "-".to_string()
            } else {
                w.current_task_id.chars().take(8).collect::<String>()
            };
            let agent = if w.current_agent.is_empty() {
                "-".to_string()
            } else {
                w.current_agent.clone()
            };

            let style = if i == app.selected_task {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(short_id),
                Cell::from(Span::styled(status_str, Style::default().fg(status_color))),
                Cell::from(task_id),
                Cell::from(agent),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(12),
        Constraint::Min(15),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!(" Workers ({}) ", ws.workers.len())));

    let mut state = TableState::default();
    if !ws.workers.is_empty() {
        state.select(Some(app.selected_task));
    }

    f.render_stateful_widget(table, chunks[1], &mut state);
}

fn render_daemon(f: &mut Frame, app: &App, area: Rect) {
    let content = match &app.daemon_info {
        None => vec![Line::from("No daemon info available")],
        Some(info) => {
            let hours = info.uptime_seconds / 3600;
            let mins = (info.uptime_seconds % 3600) / 60;
            let secs = info.uptime_seconds % 60;
            vec![
                Line::from(vec![
                    Span::styled("Version:      ", Style::default().fg(Color::Cyan)),
                    Span::raw(&info.version),
                ]),
                Line::from(vec![
                    Span::styled("Uptime:       ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{hours:02}:{mins:02}:{secs:02}")),
                ]),
                Line::from(vec![
                    Span::styled("Config Path:  ", Style::default().fg(Color::Cyan)),
                    Span::raw(&info.config_path),
                ]),
                Line::from(vec![
                    Span::styled("Workers:      ", Style::default().fg(Color::Cyan)),
                    Span::raw(info.workers_count.to_string()),
                ]),
            ]
        }
    };

    let p = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(" Daemon Info "));
    f.render_widget(p, area);
}

fn render_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(err) = &app.error {
        Line::from(Span::styled(format!(" Error: {err}"), Style::default().fg(Color::Red)))
    } else {
        Line::from(Span::styled(
            " q/Esc: quit  Tab/←→: switch tab  j/k/↑↓: navigate  r: refresh",
            Style::default().fg(Color::DarkGray),
        ))
    };
    f.render_widget(Paragraph::new(text), area);
}

fn format_issue_ref(issue_ref: &Option<crate::client::proto::IssueRef>) -> String {
    use crate::client::proto::issue_ref::Ref;
    match issue_ref {
        None => "-".to_string(),
        Some(r) => match &r.r#ref {
            None => "-".to_string(),
            Some(Ref::Github(g)) => format!("github:{}/{}/{}", g.organization, g.repository, g.number),
            Some(Ref::Centy(c)) => format!("centy:{}/{}/{}", c.organization, c.repository, c.number),
            Some(Ref::Jira(j)) => format!("jira:{}", j.id),
            Some(Ref::Link(l)) => l.url.clone(),
        },
    }
}
