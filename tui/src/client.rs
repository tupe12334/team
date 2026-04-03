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
        match resp.result {
            Some(proto::get_info_response::Result::Ok(info)) => Ok(Some(info)),
            _ => Ok(None),
        }
    }

    pub async fn list_tasks(&mut self) -> Result<Vec<Task>> {
        let resp = self
            .queue
            .list_queue(tonic::Request::new(ListQueueRequest {}))
            .await?
            .into_inner();
        match resp.result {
            Some(proto::list_queue_response::Result::Ok(task_list)) => Ok(task_list.tasks),
            _ => Ok(vec![]),
        }
    }

    pub async fn get_worker_status(&mut self) -> Result<Option<WorkerStatusData>> {
        let resp = self
            .worker
            .get_worker_status(tonic::Request::new(WorkerStatusRequest {}))
            .await?
            .into_inner();
        match resp.result {
            Some(proto::worker_status_response::Result::Ok(data)) => Ok(Some(data)),
            _ => Ok(None),
        }
    }
}
