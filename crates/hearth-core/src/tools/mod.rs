//! Tools the agent can call. Built-ins live here; MCP servers plug in through the same trait.

mod fs;

pub use fs::{ListDir, ReadFile, WriteFile};

use crate::types::ToolSpec;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Whether a tool may run without asking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Allow,
    Ask,
    Deny,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    /// Default permission when the project hasn't set one.
    fn default_permission(&self) -> Permission {
        Permission::Allow
    }
    /// Ok = result text for the model; Err = error text for the model (counts as a tool error).
    async fn call(&self, args: Value) -> Result<String, String>;
}

#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// The built-in file tools, scoped to one project folder.
    pub fn with_project_files(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let mut r = Self::new();
        r.add(Arc::new(ListDir::new(root.clone())));
        r.add(Arc::new(ReadFile::new(root.clone())));
        r.add(Arc::new(WriteFile::new(root)));
        r
    }

    pub fn add(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.spec().name, tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Sorted by name so the tool list is stable between calls (helps prompt caching).
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|t| t.spec()).collect()
    }
}
