//! Application composition: tool routing over shared execution owners.
use crate::{agents::AgentHost, control::Control, *};
pub struct Execution {
    control: Arc<Control>,
    agents: Arc<AgentHost>,
}
impl Execution {
    pub fn new(control: Arc<Control>, agents: Arc<AgentHost>) -> Arc<Self> {
        Arc::new(Self { control, agents })
    }
    pub fn approval_mode(&self) -> Result<bool> {
        self.control.approval_mode()
    }
    pub fn auto_open_codex(&self) -> Result<bool> {
        self.control.auto_open_codex()
    }
    pub async fn call(&self, name: &str, args: Value, policy: Value) -> Result<Value> {
        if name == "agent_context" {
            crate::context::read(&self.agents, &self.control, args, policy).await
        } else if name == "agents" || name.starts_with("agent_") {
            self.agents.tool(&self.control, name, args).await
        } else {
            self.control.tool(name, args).await
        }
    }
}
