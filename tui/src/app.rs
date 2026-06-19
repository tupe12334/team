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

/// Sets `slot` to `msg` only if `slot` is currently `None` (first-error-wins).
/// Extracted from `refresh()` so both the `None` (sets) and `Some` (skips) arms
/// are testable without requiring a live or mocked gRPC client.
fn record_first_error(slot: &mut Option<String>, msg: String) {
    if slot.is_none() {
        *slot = Some(msg);
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
            last_refresh: Instant::now()
                .checked_sub(Duration::from_secs(10))
                .unwrap_or_else(Instant::now),
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
                        self.last_refresh = Instant::now()
                            .checked_sub(Duration::from_secs(10))
                            .unwrap_or_else(Instant::now);
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
                record_first_error(&mut self.error, format!("Workers: {e}"));
            }
        }

        match self.client.get_daemon_info().await {
            Ok(info) => self.daemon_info = info,
            Err(e) => {
                self.daemon_info = None; // clear stale data
                record_first_error(&mut self.error, format!("Daemon: {e}"));
            }
        }

        let active_len = match self.active_tab {
            Tab::Queue => self.tasks.len(),
            Tab::Workers => self.worker_status.as_ref().map_or(0, |w| w.workers.len()),
            Tab::Daemon => 0,
        };
        self.clamp_selection(active_len);
    }

    fn clamp_selection(&mut self, active_len: usize) {
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

    /// clamp_selection must reduce selected_task to active_len-1 when the selection points
    /// past the end of the list (e.g. after the list shrinks on a refresh).
    /// This is the true arm of `if active_len > 0 && self.selected_task >= active_len`.
    /// refresh() always clears data when the daemon is unreachable → active_len=0 in all
    /// refresh tests → the true arm is never reachable there; testing clamp_selection directly
    /// covers it without needing a running daemon.
    #[tokio::test]
    async fn clamp_selection_reduces_out_of_bounds_index() {
        let mut app = make_app().await;
        app.selected_task = 5;
        app.clamp_selection(3); // active_len=3, selected_task=5 ≥ 3 → clamp to 2
        assert_eq!(app.selected_task, 2, "out-of-bounds selection must be clamped to active_len - 1");
    }

    /// When selected_task is already within bounds the guard must not fire —
    /// the `else` (false) arm: `active_len > 0 && selected_task < active_len`.
    #[tokio::test]
    async fn clamp_selection_noop_when_in_bounds() {
        let mut app = make_app().await;
        app.selected_task = 1;
        app.clamp_selection(3); // active_len=3, selected_task=1 < 3 → no-op
        assert_eq!(app.selected_task, 1, "in-bounds selection must not be changed");
    }

    /// When active_len is zero the first condition `active_len > 0` is false so the
    /// clamping must not fire even if selected_task is arbitrarily large — prevents
    /// subtraction overflow on `active_len - 1`.
    #[tokio::test]
    async fn clamp_selection_noop_when_active_len_zero() {
        let mut app = make_app().await;
        app.selected_task = 5;
        app.clamp_selection(0); // active_len=0 → guard false → no-op
        assert_eq!(app.selected_task, 5, "selection must not be changed when active_len is zero");
    }

    /// select_next with exactly one task: (0 + 1) % 1 = 0 — the selection must stay at 0.
    /// Existing tests cover len=0 (guard false, no-op) and len=2 (position changes).
    /// len=1 is the boundary between the two: the guard fires (len > 0) but the modulo
    /// arithmetic produces the same index — explicitly asserting this prevents a future
    /// off-by-one regression (e.g. accidentally using `% (len - 1)` or `% (len + 1)`).
    #[tokio::test]
    async fn select_next_single_item_noop_in_queue_tab() {
        let mut app = make_app().await;
        app.tasks = vec![crate::client::Task { id: "only".into(), ..Default::default() }];
        app.selected_task = 0;
        app.select_next();
        assert_eq!(app.selected_task, 0, "select_next with a single-item list must keep selection at 0");
    }

    /// select_prev with exactly one task: (0 + 1 - 1) % 1 = 0 % 1 = 0 — same invariant.
    /// This is the symmetric case for select_prev; tests len=1 where neither the guard
    /// (len > 0 is true) nor the arithmetic (0 % 1 = 0) changes the index.
    #[tokio::test]
    async fn select_prev_single_item_noop_in_queue_tab() {
        let mut app = make_app().await;
        app.tasks = vec![crate::client::Task { id: "only".into(), ..Default::default() }];
        app.selected_task = 0;
        app.select_prev();
        assert_eq!(app.selected_task, 0, "select_prev with a single-item list must keep selection at 0");
    }

    // --- record_first_error tests ---

    /// When the slot is None, record_first_error must set it to the provided message.
    /// This is the `true` arm of `if slot.is_none()` — the only arm exercised in the
    /// Workers/Daemon error paths of `refresh()` when Queue has already succeeded and
    /// left `self.error` as None.
    #[test]
    fn record_first_error_sets_slot_when_none() {
        let mut slot: Option<String> = None;
        record_first_error(&mut slot, "Workers: connect failed".into());
        assert_eq!(slot.as_deref(), Some("Workers: connect failed"));
    }

    /// When the slot is already Some, record_first_error must not overwrite it.
    /// This is the `false` arm of `if slot.is_none()` — the path taken when Queue
    /// already set an error and Workers/Daemon errors are secondary.
    #[test]
    fn record_first_error_skips_when_slot_already_set() {
        let mut slot: Option<String> = Some("Queue: connect failed".into());
        record_first_error(&mut slot, "Workers: also failed".into());
        assert_eq!(
            slot.as_deref(),
            Some("Queue: connect failed"),
            "first error must not be overwritten by a subsequent error"
        );
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
