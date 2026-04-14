use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::client::{Client, DaemonInfo, Task, WorkerStatusData};
use crate::ui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Queue,
    Workers,
    Daemon,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Queue, Tab::Workers, Tab::Daemon];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Queue => "Queue",
            Tab::Workers => "Workers",
            Tab::Daemon => "Daemon",
        }
    }
}

pub struct App {
    pub client: Client,
    pub active_tab: Tab,
    pub tasks: Vec<Task>,
    pub worker_status: Option<WorkerStatusData>,
    pub daemon_info: Option<DaemonInfo>,
    pub error: Option<String>,
    pub selected_task: usize,
    last_refresh: Instant,
}

impl App {
    pub fn new(addr: String) -> Result<Self> {
        let client = Client::new(addr)?;
        Ok(Self {
            client,
            active_tab: Tab::Queue,
            tasks: vec![],
            worker_status: None,
            daemon_info: None,
            error: None,
            selected_task: 0,
            last_refresh: Instant::now() - Duration::from_secs(10),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = ratatui::init();
        let result = self.event_loop(&mut terminal).await;
        ratatui::restore();
        result
    }

    async fn event_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            if self.last_refresh.elapsed() >= Duration::from_secs(2) {
                self.refresh().await;
            }

            terminal.draw(|f| ui::render(f, self))?;

            if event::poll(Duration::from_millis(250))?
                && let Event::Key(key) = event::read()?
            {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => self.next_tab(),
                    KeyCode::BackTab | KeyCode::Char('h') | KeyCode::Left => {
                        self.prev_tab();
                    }
                    KeyCode::Char('j') | KeyCode::Down => self.select_next(),
                    KeyCode::Char('k') | KeyCode::Up => self.select_prev(),
                    KeyCode::Char('r') => {
                        self.last_refresh =
                            Instant::now() - Duration::from_secs(10);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    async fn refresh(&mut self) {
        self.last_refresh = Instant::now();
        self.error = None;

        match self.client.list_tasks().await {
            Ok(tasks) => self.tasks = tasks,
            Err(e) => {
                self.tasks = vec![]; // clear stale data so UI shows error, not outdated tasks
                self.error = Some(format!("Queue: {e}"));
            }
        }

        match self.client.get_worker_status().await {
            Ok(status) => self.worker_status = status,
            Err(e) => {
                self.worker_status = None; // clear stale data
                if self.error.is_none() {
                    self.error = Some(format!("Workers: {e}"));
                }
            }
        }

        match self.client.get_daemon_info().await {
            Ok(info) => self.daemon_info = info,
            Err(e) => {
                self.daemon_info = None; // clear stale data
                if self.error.is_none() {
                    self.error = Some(format!("Daemon: {e}"));
                }
            }
        }

        let active_len = match self.active_tab {
            Tab::Queue => self.tasks.len(),
            Tab::Workers => self.worker_status.as_ref().map_or(0, |w| w.workers.len()),
            Tab::Daemon => 0,
        };
        if active_len > 0 && self.selected_task >= active_len {
            self.selected_task = active_len - 1;
        }
    }

    fn next_tab(&mut self) {
        let idx = Tab::ALL.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        self.active_tab = Tab::ALL[(idx + 1) % Tab::ALL.len()];
        self.selected_task = 0;
    }

    fn prev_tab(&mut self) {
        let idx = Tab::ALL.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        self.active_tab = Tab::ALL[(idx + Tab::ALL.len() - 1) % Tab::ALL.len()];
        self.selected_task = 0;
    }

    fn select_next(&mut self) {
        let len = match self.active_tab {
            Tab::Queue => self.tasks.len(),
            Tab::Workers => {
                self.worker_status.as_ref().map_or(0, |w| w.workers.len())
            }
            Tab::Daemon => 0,
        };
        if len > 0 {
            self.selected_task = (self.selected_task + 1) % len;
        }
    }

    fn select_prev(&mut self) {
        let len = match self.active_tab {
            Tab::Queue => self.tasks.len(),
            Tab::Workers => {
                self.worker_status.as_ref().map_or(0, |w| w.workers.len())
            }
            Tab::Daemon => 0,
        };
        if len > 0 {
            self.selected_task = (self.selected_task + len - 1) % len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_app() -> App {
        App::new("http://[::1]:50051".to_string()).expect("failed to create app")
    }

    #[tokio::test]
    async fn next_tab_cycles_forward() {
        let mut app = make_app().await;
        assert_eq!(app.active_tab, Tab::Queue);
        app.next_tab();
        assert_eq!(app.active_tab, Tab::Workers);
        app.next_tab();
        assert_eq!(app.active_tab, Tab::Daemon);
        app.next_tab();
        assert_eq!(app.active_tab, Tab::Queue); // wraps
    }

    #[tokio::test]
    async fn prev_tab_cycles_backward() {
        let mut app = make_app().await;
        assert_eq!(app.active_tab, Tab::Queue);
        app.prev_tab();
        assert_eq!(app.active_tab, Tab::Daemon); // wraps
        app.prev_tab();
        assert_eq!(app.active_tab, Tab::Workers);
    }

    #[tokio::test]
    async fn next_tab_resets_selection() {
        let mut app = make_app().await;
        app.selected_task = 3;
        app.next_tab();
        assert_eq!(app.selected_task, 0);
    }

    #[tokio::test]
    async fn prev_tab_resets_selection() {
        // next_tab_resets_selection tests next_tab(); this mirrors it for prev_tab().
        // prev_tab() calls `self.selected_task = 0` at line 146 — not covered by
        // prev_tab_cycles_backward which only checks active_tab.
        let mut app = make_app().await;
        app.selected_task = 3;
        app.prev_tab();
        assert_eq!(app.selected_task, 0);
    }

    #[tokio::test]
    async fn select_next_wraps_in_queue_tab() {
        let mut app = make_app().await;
        app.tasks = vec![
            crate::client::Task { id: "t1".into(), ..Default::default() },
            crate::client::Task { id: "t2".into(), ..Default::default() },
        ];
        app.selected_task = 0;
        app.select_next();
        assert_eq!(app.selected_task, 1);
        app.select_next();
        assert_eq!(app.selected_task, 0); // wraps
    }

    #[tokio::test]
    async fn select_prev_wraps_in_queue_tab() {
        let mut app = make_app().await;
        app.tasks = vec![
            crate::client::Task { id: "t1".into(), ..Default::default() },
            crate::client::Task { id: "t2".into(), ..Default::default() },
        ];
        app.selected_task = 0;
        app.select_prev();
        assert_eq!(app.selected_task, 1); // wraps to last
    }

    #[tokio::test]
    async fn select_next_noop_when_empty() {
        let mut app = make_app().await;
        app.selected_task = 0;
        app.select_next();
        assert_eq!(app.selected_task, 0);
    }

    #[tokio::test]
    async fn select_prev_noop_when_empty() {
        let mut app = make_app().await;
        app.selected_task = 0;
        app.select_prev();
        assert_eq!(app.selected_task, 0);
    }

    #[tokio::test]
    async fn select_next_wraps_in_workers_tab() {
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        app.worker_status = Some(crate::client::WorkerStatusData {
            total: 4,
            busy: 2,
            idle: 2,
            workers: vec![
                crate::client::proto::WorkerInfo { worker_id: "w1".into(), ..Default::default() },
                crate::client::proto::WorkerInfo { worker_id: "w2".into(), ..Default::default() },
            ],
        });
        app.selected_task = 0;
        app.select_next();
        assert_eq!(app.selected_task, 1);
        app.select_next();
        assert_eq!(app.selected_task, 0); // wraps
    }

    #[tokio::test]
    async fn select_prev_wraps_in_workers_tab() {
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        app.worker_status = Some(crate::client::WorkerStatusData {
            total: 4,
            busy: 1,
            idle: 3,
            workers: vec![
                crate::client::proto::WorkerInfo { worker_id: "w1".into(), ..Default::default() },
                crate::client::proto::WorkerInfo { worker_id: "w2".into(), ..Default::default() },
            ],
        });
        app.selected_task = 0;
        app.select_prev();
        assert_eq!(app.selected_task, 1); // wraps to last
    }

    #[tokio::test]
    async fn select_next_noop_on_daemon_tab() {
        let mut app = make_app().await;
        app.active_tab = Tab::Daemon;
        app.selected_task = 2;
        app.select_next();
        assert_eq!(app.selected_task, 2, "select_next must be a no-op on Daemon tab (len=0)");
    }

    #[tokio::test]
    async fn select_prev_noop_on_daemon_tab() {
        let mut app = make_app().await;
        app.active_tab = Tab::Daemon;
        app.selected_task = 2;
        app.select_prev();
        assert_eq!(app.selected_task, 2, "select_prev must be a no-op on Daemon tab (len=0)");
    }

    #[tokio::test]
    async fn select_next_noop_when_workers_tab_no_status() {
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        // worker_status is None → map_or(0, ...) = 0
        app.selected_task = 0;
        app.select_next();
        assert_eq!(app.selected_task, 0, "select_next must be a no-op when worker_status is None");
    }

    #[tokio::test]
    async fn select_prev_noop_when_workers_tab_no_status() {
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        // worker_status is None → map_or(0, ...) = 0
        app.selected_task = 0;
        app.select_prev();
        assert_eq!(app.selected_task, 0, "select_prev must be a no-op when worker_status is None");
    }

    #[tokio::test]
    async fn refresh_clears_worker_status_when_daemon_unreachable() {
        let mut app = make_app().await;
        app.worker_status = Some(crate::client::WorkerStatusData {
            total: 4,
            busy: 1,
            idle: 3,
            workers: vec![],
        });
        app.refresh().await;
        assert!(app.worker_status.is_none(), "stale worker status must be cleared on daemon error");
    }

    #[test]
    fn tab_titles_are_correct() {
        assert_eq!(Tab::Queue.title(), "Queue");
        assert_eq!(Tab::Workers.title(), "Workers");
        assert_eq!(Tab::Daemon.title(), "Daemon");
    }

    #[tokio::test]
    async fn refresh_clears_tasks_when_daemon_unreachable() {
        let mut app = make_app().await;
        app.tasks = vec![crate::client::Task { id: "stale".into(), ..Default::default() }];
        app.refresh().await;
        assert!(app.tasks.is_empty(), "stale tasks must be cleared on daemon error");
        assert!(app.error.is_some(), "error must be set when daemon is unreachable");
    }

    #[tokio::test]
    async fn refresh_clears_daemon_info_when_daemon_unreachable() {
        let mut app = make_app().await;
        app.daemon_info = Some(crate::client::DaemonInfo {
            version: "old".into(),
            uptime_seconds: 999,
            config_path: "/old".into(),
            workers_count: 4,
        });
        app.refresh().await;
        assert!(app.daemon_info.is_none(), "stale daemon info must be cleared on error");
    }

    /// refresh() computes active_len via a match on active_tab.  All existing refresh tests
    /// use the default Tab::Queue arm; this test exercises the Tab::Workers arm specifically.
    /// When the daemon is unavailable, get_worker_status() clears worker_status to None,
    /// so active_len = map_or(0, ...) = 0 and the selection-clamp guard stays false.
    #[tokio::test]
    async fn refresh_active_len_uses_workers_when_on_workers_tab() {
        let mut app = make_app().await;
        app.active_tab = Tab::Workers;
        app.selected_task = 0;
        app.refresh().await;
        // Daemon unreachable → worker_status cleared to None → Tab::Workers arm returns 0
        assert!(app.worker_status.is_none());
        // active_len = 0 → clamping condition `active_len > 0` is false → selection unchanged
        assert_eq!(app.selected_task, 0);
    }

    /// refresh() computes active_len via a match on active_tab.  All existing refresh tests
    /// use the default Tab::Queue arm; this test exercises the Tab::Daemon arm specifically
    /// (Tab::Daemon => 0), verifying that high selection indices are not clamped when
    /// active_len is always zero on the Daemon tab.
    #[tokio::test]
    async fn refresh_active_len_is_zero_on_daemon_tab() {
        let mut app = make_app().await;
        app.active_tab = Tab::Daemon;
        app.selected_task = 5; // deliberately high — must not be clamped
        app.refresh().await;
        // Tab::Daemon => active_len = 0 → clamping guard `active_len > 0` is false
        assert_eq!(app.selected_task, 5, "selection must not be clamped on Daemon tab");
    }
}
