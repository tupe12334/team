use color_eyre::Result;
use tonic::transport::Channel;

pub mod proto {
    tonic::include_proto!("team");
}

pub use proto::{
    DaemonInfo, Task, TaskStatus, WorkerStatus, WorkerStatusData,
    daemon_service_client::DaemonServiceClient, queue_service_client::QueueServiceClient,
    worker_service_client::WorkerServiceClient,
};

use proto::{ListQueueRequest, WorkerStatusRequest};

pub struct Client {
    daemon: DaemonServiceClient<Channel>,
    queue: QueueServiceClient<Channel>,
    worker: WorkerServiceClient<Channel>,
}

impl Client {
    pub fn new(addr: String) -> Result<Self> {
        let channel = Channel::from_shared(addr)?.connect_lazy();
        Ok(Self {
            daemon: DaemonServiceClient::new(channel.clone()),
            queue: QueueServiceClient::new(channel.clone()),
            worker: WorkerServiceClient::new(channel),
        })
    }

    pub async fn get_daemon_info(&mut self) -> Result<Option<DaemonInfo>> {
        let resp = self
            .daemon
            .get_info(tonic::Request::new(()))
            .await?
            .into_inner();
        parse_daemon_info(resp.result)
    }

    pub async fn list_tasks(&mut self) -> Result<Vec<Task>> {
        let resp = self
            .queue
            .list_queue(tonic::Request::new(ListQueueRequest {}))
            .await?
            .into_inner();
        parse_task_list(resp.result)
    }

    pub async fn get_worker_status(&mut self) -> Result<Option<WorkerStatusData>> {
        let resp = self
            .worker
            .get_worker_status(tonic::Request::new(WorkerStatusRequest {}))
            .await?
            .into_inner();
        parse_worker_status(resp.result)
    }
}

fn parse_daemon_info(result: Option<proto::get_info_response::Result>) -> Result<Option<DaemonInfo>> {
    match result {
        Some(proto::get_info_response::Result::Ok(info)) => Ok(Some(info)),
        Some(proto::get_info_response::Result::Error(msg)) => {
            Err(color_eyre::eyre::eyre!("{msg}"))
        }
        None => Ok(None),
    }
}

fn parse_task_list(result: Option<proto::list_queue_response::Result>) -> Result<Vec<Task>> {
    match result {
        Some(proto::list_queue_response::Result::Ok(task_list)) => Ok(task_list.tasks),
        Some(proto::list_queue_response::Result::Error(msg)) => {
            Err(color_eyre::eyre::eyre!("{msg}"))
        }
        None => Ok(vec![]),
    }
}

fn parse_worker_status(result: Option<proto::worker_status_response::Result>) -> Result<Option<WorkerStatusData>> {
    match result {
        Some(proto::worker_status_response::Result::Ok(data)) => Ok(Some(data)),
        Some(proto::worker_status_response::Result::Error(msg)) => {
            Err(color_eyre::eyre::eyre!("{msg}"))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_daemon_info ---

    #[test]
    fn daemon_info_ok_arm_returns_some() {
        let info = DaemonInfo { version: "1.0".into(), uptime_seconds: 42, config_path: "/etc/team".into(), workers_count: 4 };
        let result = parse_daemon_info(Some(proto::get_info_response::Result::Ok(info.clone())));
        assert_eq!(result.unwrap(), Some(info));
    }

    #[test]
    fn daemon_info_error_arm_returns_err() {
        let result = parse_daemon_info(Some(proto::get_info_response::Result::Error("boom".into())));
        let err = result.unwrap_err();
        assert!(err.to_string().contains("boom"), "error message must propagate, got: {err}");
    }

    #[test]
    fn daemon_info_none_arm_returns_ok_none() {
        let result = parse_daemon_info(None);
        assert!(result.unwrap().is_none(), "None result must map to Ok(None)");
    }

    // --- parse_task_list ---

    #[test]
    fn task_list_ok_arm_returns_tasks() {
        let task = Task { id: "t1".into(), ..Default::default() };
        let task_list = proto::TaskList { tasks: vec![task.clone()] };
        let result = parse_task_list(Some(proto::list_queue_response::Result::Ok(task_list)));
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "t1");
    }

    #[test]
    fn task_list_error_arm_returns_err() {
        let result = parse_task_list(Some(proto::list_queue_response::Result::Error("queue error".into())));
        let err = result.unwrap_err();
        assert!(err.to_string().contains("queue error"), "got: {err}");
    }

    #[test]
    fn task_list_none_arm_returns_empty_vec() {
        let result = parse_task_list(None);
        assert!(result.unwrap().is_empty(), "None result must map to Ok(vec![])");
    }

    // --- parse_worker_status ---

    #[test]
    fn worker_status_ok_arm_returns_some() {
        let data = WorkerStatusData { total: 4, busy: 1, idle: 3, workers: vec![] };
        let result = parse_worker_status(Some(proto::worker_status_response::Result::Ok(data.clone())));
        let got = result.unwrap().unwrap();
        assert_eq!(got.total, 4);
        assert_eq!(got.busy, 1);
    }

    #[test]
    fn worker_status_error_arm_returns_err() {
        let result = parse_worker_status(Some(proto::worker_status_response::Result::Error("worker error".into())));
        let err = result.unwrap_err();
        assert!(err.to_string().contains("worker error"), "got: {err}");
    }

    #[test]
    fn worker_status_none_arm_returns_ok_none() {
        let result = parse_worker_status(None);
        assert!(result.unwrap().is_none(), "None result must map to Ok(None)");
    }
}
