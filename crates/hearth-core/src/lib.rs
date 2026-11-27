//! Hearth core: providers, the agent loop, tools, escalation and storage.
//! No UI code lives here, so it can be tested headless and reused by a server build later.

pub mod agent;
pub mod escalation;
pub mod models;
pub mod provider;
pub mod store;
pub mod tools;
pub mod types;

pub use agent::{AgentEvent, AgentHooks, AgentRun, Outcome, run};
pub use escalation::{EscalationPolicy, StallReason};
pub use models::{LocalModel, Role, RoleDefaults};
pub use provider::{Provider, ProviderConfig, ProviderError, ProviderKind};
pub use store::{Project, Store, Thread, ThreadStatus};
pub use tools::{Permission, Tool, ToolRegistry};
pub use types::*;
