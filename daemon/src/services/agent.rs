use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::gstack_agents;
use crate::proto::agent_service_server::AgentService;
use crate::proto::{
    get_available_agents_response, AgentInfo, AgentList, GetAvailableAgentsResponse,
};
use crate::state::AppState;

pub struct AgentServiceImpl {
    state: Arc<Mutex<AppState>>,
}

impl AgentServiceImpl {
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl AgentService for AgentServiceImpl {
    async fn get_available_agents(
        &self,
        _request: Request<()>,
    ) -> Result<Response<GetAvailableAgentsResponse>, Status> {
        let state = self.state.lock().await;
        let agents = gstack_agents::filter(&state.config.enabled_agents)
            .into_iter()
            .map(|a| AgentInfo {
                name: a.name.to_string(),
                description: a.description.to_string(),
            })
            .collect();
        Ok(Response::new(GetAvailableAgentsResponse {
            result: Some(get_available_agents_response::Result::Ok(AgentList {
                agents,
            })),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{DaemonConfig, agent_service_server::AgentService};

    fn make_state(enabled_agents: Vec<String>) -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState {
            config_path: "/tmp/agent-svc-test.toml".into(),
            queue_path: "/tmp/agent-svc-test.json".into(),
            queue: Vec::new(),
            workers: Vec::new(),
            config: DaemonConfig {
                workers_count: 4,
                log_level: "info".into(),
                enabled_agents,
            },
        }))
    }

    #[tokio::test]
    async fn returns_all_35_agents_when_no_filter() {
        let svc = AgentServiceImpl::new(make_state(vec![]));
        let res = svc.get_available_agents(Request::new(())).await.unwrap().into_inner();
        let agents = match res.result.unwrap() {
            get_available_agents_response::Result::Ok(list) => list.agents,
            get_available_agents_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(agents.len(), 35);
    }

    #[tokio::test]
    async fn filters_agents_to_enabled_subset() {
        let enabled = vec!["review".to_string(), "qa".to_string(), "ship".to_string()];
        let svc = AgentServiceImpl::new(make_state(enabled));
        let res = svc.get_available_agents(Request::new(())).await.unwrap().into_inner();
        let agents = match res.result.unwrap() {
            get_available_agents_response::Result::Ok(list) => list.agents,
            get_available_agents_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert_eq!(agents.len(), 3);
        let names: Vec<&str> = agents.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"review"));
        assert!(names.contains(&"qa"));
        assert!(names.contains(&"ship"));
    }

    #[tokio::test]
    async fn all_returned_agents_have_nonempty_descriptions() {
        let svc = AgentServiceImpl::new(make_state(vec![]));
        let res = svc.get_available_agents(Request::new(())).await.unwrap().into_inner();
        let agents = match res.result.unwrap() {
            get_available_agents_response::Result::Ok(list) => list.agents,
            get_available_agents_response::Result::Error(e) => panic!("unexpected error: {e}"),
        };
        assert!(agents.iter().all(|a| !a.description.is_empty()));
    }
}
