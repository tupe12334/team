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
            let (status_str, status_color) = match TaskStatus::try_from(task.status).unwrap_or(TaskStatus::Queued) {
                TaskStatus::Queued => ("queued", Color::White),
                TaskStatus::Running => ("running", Color::Green),
                TaskStatus::Completed => ("completed", Color::Blue),
                TaskStatus::Failed => ("failed", Color::Red),
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
                Cell::from(task.agent.as_deref().unwrap_or("")),
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
            Some(Ref::Github(g)) => format!("github:{}/{}#{}", g.organization, g.repository, g.number),
            Some(Ref::Centy(c)) => format!("centy:{}/{}#{}", c.organization, c.repository, c.number),
            Some(Ref::Jira(j)) => format!("jira:{}", j.id),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use crate::app::{App, Tab};
    use crate::client::{DaemonInfo, WorkerStatusData, Task, TaskStatus, WorkerStatus};
    use crate::client::proto::{issue_ref, CentyIssueRef, GitHubIssueRef, IssueRef, JiraIssueRef, WorkerInfo};

    async fn make_app() -> App {
        App::new("http://[::1]:50051".to_string()).expect("failed to create app")
    }

    fn make_terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(80, 30)).unwrap()
    }

    fn buffer_has(terminal: &Terminal<TestBackend>, text: &str) -> bool {
        let buf = terminal.backend().buffer();
        let area = buf.area();
        (0..area.height).any(|y| {
            let line: String = (0..area.width)
                .map(|x| buf.cell((x, y)).map_or(" ".to_string(), |c| c.symbol().to_string()))
                .collect();
            line.contains(text)
        })
    }

    fn github(org: &str, repo: &str, number: &str) -> Option<IssueRef> {
        Some(IssueRef { r#ref: Some(issue_ref::Ref::Github(GitHubIssueRef {
            organization: org.into(), repository: repo.into(), number: number.into(),
        })) })
    }
    fn centy(org: &str, repo: &str, number: &str) -> Option<IssueRef> {
        Some(IssueRef { r#ref: Some(issue_ref::Ref::Centy(CentyIssueRef {
            organization: org.into(), repository: repo.into(), number: number.into(),
        })) })
    }
    fn jira(id: &str) -> Option<IssueRef> {
        Some(IssueRef { r#ref: Some(issue_ref::Ref::Jira(JiraIssueRef { id: id.into() })) })
    }

    #[test]
    fn format_none_returns_dash() {
        assert_eq!(format_issue_ref(&None), "-");
    }

    #[test]
    fn format_empty_inner_ref_returns_dash() {
        assert_eq!(format_issue_ref(&Some(IssueRef { r#ref: None })), "-");
    }

    #[test]
    fn format_github() {
        assert_eq!(format_issue_ref(&github("acme", "app", "42")), "github:acme/app#42");
    }

    #[test]
    fn format_centy() {
        assert_eq!(format_issue_ref(&centy("acme", "proj", "7")), "centy:acme/proj#7");
    }

    #[test]
    fn format_jira() {
        assert_eq!(format_issue_ref(&jira("PROJ-123")), "jira:PROJ-123");
    }

    #[tokio::test]
    async fn render_daemon_shows_placeholder_when_daemon_info_is_none() {
        let mut app = make_app().await;
        app.active_tab = Tab::Daemon;
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "No daemon info available"), "None arm must render placeholder");
    }

    #[tokio::test]
    async fn render_daemon_shows_version_and_uptime_when_some() {
        let mut app = make_app().await;
        app.active_tab = Tab::Daemon;
        app.daemon_info = Some(DaemonInfo {
            version: "1.2.3".into(),
            uptime_seconds: 3725, // 01:02:05
            config_path: "/etc/daemon.toml".into(),
            workers_count: 4,
        });
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "1.2.3"), "version must appear in daemon info");
        assert!(buffer_has(&terminal, "01:02:05"), "uptime must be formatted as HH:MM:SS");
    }

    #[tokio::test]
    async fn render_workers_shows_placeholder_when_worker_status_is_none() {
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "No worker data available"), "None arm must render placeholder");
    }

    #[tokio::test]
    async fn render_workers_idle_worker_shows_idle_and_dashes_for_empty_fields() {
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        app.worker_status = Some(WorkerStatusData {
            total: 1,
            busy: 0,
            idle: 1,
            workers: vec![WorkerInfo {
                worker_id: "w-idle".into(),
                status: WorkerStatus::Idle as i32,
                current_task_id: "".into(),
                current_agent: "".into(),
                ..Default::default()
            }],
        });
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "idle"), "idle worker must render 'idle'");
        assert!(buffer_has(&terminal, "-"), "empty task_id and agent must render as '-'");
    }

    #[tokio::test]
    async fn render_workers_busy_worker_shows_busy_and_agent() {
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        app.worker_status = Some(WorkerStatusData {
            total: 1,
            busy: 1,
            idle: 0,
            workers: vec![WorkerInfo {
                worker_id: "w-busy".into(),
                status: WorkerStatus::Busy as i32,
                current_task_id: "task-abc".into(),
                current_agent: "review".into(),
                ..Default::default()
            }],
        });
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "busy"), "busy worker must render 'busy'");
        assert!(buffer_has(&terminal, "review"), "current agent must appear in the row");
    }

    #[tokio::test]
    async fn render_statusbar_shows_error_text_when_error_is_some() {
        let mut app = make_app().await;
        app.error = Some("daemon unreachable".into());
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "Error: daemon unreachable"), "error must appear in status bar");
    }

    #[tokio::test]
    async fn render_statusbar_shows_keybinding_hints_when_no_error() {
        let app = make_app().await;
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "quit"), "keybinding hints must appear when no error");
    }

    #[tokio::test]
    async fn render_queue_shows_running_label() {
        let mut app = make_app().await;
        app.tasks = vec![Task { id: "t1".into(), status: TaskStatus::Running as i32, ..Default::default() }];
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "running"), "Running task must render 'running'");
    }

    #[tokio::test]
    async fn render_queue_shows_completed_label() {
        let mut app = make_app().await;
        app.tasks = vec![Task { id: "t2".into(), status: TaskStatus::Completed as i32, ..Default::default() }];
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "completed"), "Completed task must render 'completed'");
    }

    #[tokio::test]
    async fn render_queue_shows_failed_label() {
        let mut app = make_app().await;
        app.tasks = vec![Task { id: "t3".into(), status: TaskStatus::Failed as i32, ..Default::default() }];
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "failed"), "Failed task must render 'failed'");
    }

    #[tokio::test]
    async fn render_queue_shows_queued_label() {
        let mut app = make_app().await;
        app.tasks = vec![Task { id: "t4".into(), status: TaskStatus::Queued as i32, ..Default::default() }];
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "queued"), "Queued task must render 'queued'");
    }

    #[tokio::test]
    async fn render_queue_shows_agent_name_when_task_has_agent() {
        let mut app = make_app().await;
        app.tasks = vec![Task { id: "t5".into(), agent: Some("review".into()), ..Default::default() }];
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "review"), "task.agent Some value must appear in agent column");
    }

    #[tokio::test]
    async fn render_queue_empty_tasks_does_not_crash_and_shows_zero_count() {
        let app = make_app().await;
        // tasks is empty by default; state.select is NOT called (the if !app.tasks.is_empty() guard is false)
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "Queue (0 tasks)"), "empty queue must show 0 task count");
    }

    #[tokio::test]
    async fn render_workers_some_with_empty_workers_list_does_not_crash() {
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        // ws.workers is empty; state.select is NOT called (the if !ws.workers.is_empty() guard is false)
        app.worker_status = Some(WorkerStatusData { total: 0, busy: 0, idle: 0, workers: vec![] });
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "Workers (0)"), "empty worker list must show 0 count");
    }

    #[tokio::test]
    async fn render_queue_unknown_status_falls_back_to_queued() {
        // TaskStatus::try_from(99) returns Err; unwrap_or(TaskStatus::Queued) picks the fallback.
        // The row must still render with the "queued" label instead of panicking.
        let mut app = make_app().await;
        app.tasks = vec![Task { id: "t-bad".into(), status: 99, ..Default::default() }];
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "queued"), "invalid status must fall back to Queued and render 'queued'");
    }

    #[tokio::test]
    async fn render_workers_unknown_status_falls_back_to_idle() {
        // WorkerStatus::try_from(99) returns Err; unwrap_or(WorkerStatus::Idle) picks the fallback.
        // The row must still render with the "idle" label instead of panicking.
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        app.worker_status = Some(WorkerStatusData {
            total: 1,
            busy: 0,
            idle: 1,
            workers: vec![WorkerInfo {
                worker_id: "w-bad".into(),
                status: 99,
                ..Default::default()
            }],
        });
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "idle"), "invalid worker status must fall back to Idle and render 'idle'");
    }

    #[tokio::test]
    async fn render_queue_non_selected_row_uses_default_style() {
        // All existing queue tests use exactly 1 task so i==0==selected_task — the `else` branch is never reached.
        // With 2 tasks and selected_task=0, the row at index 1 takes the `else { Style::default() }` arm.
        let mut app = make_app().await;
        app.tasks = vec![
            Task { id: "sel-task-111".into(), status: TaskStatus::Queued as i32, ..Default::default() },
            Task { id: "oth-task-222".into(), status: TaskStatus::Queued as i32, ..Default::default() },
        ];
        app.selected_task = 0;
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "sel-task"), "selected task must render");
        assert!(buffer_has(&terminal, "oth-task"), "non-selected task must also render (exercises else branch)");
    }

    #[tokio::test]
    async fn render_workers_non_selected_worker_uses_default_style() {
        // All existing worker tests use exactly 1 worker so i==0==selected_task — the `else` branch is never reached.
        // With 2 workers and selected_task=0, the worker at index 1 takes the `else { Style::default() }` arm.
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        app.worker_status = Some(WorkerStatusData {
            total: 2,
            busy: 1,
            idle: 1,
            workers: vec![
                WorkerInfo {
                    worker_id: "sel-worker-1".into(),
                    status: WorkerStatus::Busy as i32,
                    current_task_id: "t1".into(),
                    current_agent: "review".into(),
                    ..Default::default()
                },
                WorkerInfo {
                    worker_id: "oth-worker-2".into(),
                    status: WorkerStatus::Idle as i32,
                    current_task_id: "".into(),
                    current_agent: "".into(),
                    ..Default::default()
                },
            ],
        });
        app.selected_task = 0;
        let mut terminal = make_terminal();
        terminal.draw(|f| render(f, &app)).unwrap();
        assert!(buffer_has(&terminal, "sel-work"), "selected worker must render");
        assert!(buffer_has(&terminal, "oth-work"), "non-selected worker must render (exercises else branch)");
    }
}
