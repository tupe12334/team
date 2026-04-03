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
