//! File tools confined to the project folder.

use super::Tool;
use crate::types::ToolSpec;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

const MAX_READ_BYTES: usize = 200_000;

/// Resolve `rel` inside `root`, refusing absolute paths and anything that climbs out.
pub(crate) fn resolve(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel = rel.trim();
    let p = Path::new(if rel.is_empty() { "." } else { rel });
    if p.is_absolute() {
        return Err(format!("path must be relative to the project folder: {rel}"));
    }
    let mut out = root.to_path_buf();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::Normal(s) => out.push(s),
            Component::ParentDir => {
                if !out.pop() || !out.starts_with(root) {
                    return Err(format!("path leaves the project folder: {rel}"));
                }
            }
            _ => return Err(format!("unsupported path: {rel}")),
        }
    }
    if !out.starts_with(root) {
        return Err(format!("path leaves the project folder: {rel}"));
    }
    // Symlinks could still point outside; check the real location of what exists.
    if let Ok(real) = out.canonicalize() {
        let real_root = root.canonicalize().map_err(|e| e.to_string())?;
        if !real.starts_with(&real_root) {
            return Err(format!("path leaves the project folder: {rel}"));
        }
    }
    Ok(out)
}

fn path_arg(args: &Value) -> Result<&str, String> {
    args["path"].as_str().ok_or_else(|| "missing required argument: path".to_string())
}

pub struct ListDir {
    root: PathBuf,
}
impl ListDir {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl Tool for ListDir {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_dir".into(),
            description: "List files and folders in a directory of the project folder. Use \".\" for the top level.".into(),
            parameters: json!({"type": "object", "properties": {"path": {"type": "string", "description": "Directory relative to the project folder"}}, "required": ["path"]}),
        }
    }
    async fn call(&self, args: Value) -> Result<String, String> {
        let dir = resolve(&self.root, args["path"].as_str().unwrap_or("."))?;
        let mut rd = tokio::fs::read_dir(&dir).await.map_err(|e| format!("cannot list {}: {e}", dir.display()))?;
        let mut names = Vec::new();
        while let Some(e) = rd.next_entry().await.map_err(|e| e.to_string())? {
            let is_dir = e.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
            names.push(format!("{}{}", e.file_name().to_string_lossy(), if is_dir { "/" } else { "" }));
        }
        names.sort();
        Ok(if names.is_empty() { "(empty)".into() } else { names.join("\n") })
    }
}

pub struct ReadFile {
    root: PathBuf,
}
impl ReadFile {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl Tool for ReadFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_file".into(),
            description: "Read a text file from the project folder.".into(),
            parameters: json!({"type": "object", "properties": {"path": {"type": "string", "description": "File path relative to the project folder"}}, "required": ["path"]}),
        }
    }
    async fn call(&self, args: Value) -> Result<String, String> {
        let p = resolve(&self.root, path_arg(&args)?)?;
        let bytes = tokio::fs::read(&p).await.map_err(|e| format!("cannot read {}: {e}", p.display()))?;
        let truncated = bytes.len() > MAX_READ_BYTES;
        let mut text = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_READ_BYTES)]).into_owned();
        if truncated {
            text.push_str(&format!("\n\n[truncated: file is {} bytes, showing the first {MAX_READ_BYTES}]", bytes.len()));
        }
        Ok(text)
    }
}

pub struct WriteFile {
    root: PathBuf,
}
impl WriteFile {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl Tool for WriteFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_file".into(),
            description: "Create or overwrite a text file in the project folder. Parent folders are created.".into(),
            parameters: json!({"type": "object", "properties": {
                "path": {"type": "string", "description": "File path relative to the project folder"},
                "content": {"type": "string", "description": "Full file contents"}
            }, "required": ["path", "content"]}),
        }
    }
    async fn call(&self, args: Value) -> Result<String, String> {
        let p = resolve(&self.root, path_arg(&args)?)?;
        let content = args["content"].as_str().ok_or("missing required argument: content")?;
        if let Some(parent) = p.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        }
        tokio::fs::write(&p, content).await.map_err(|e| format!("cannot write {}: {e}", p.display()))?;
        Ok(format!("wrote {} bytes to {}", content.len(), args["path"].as_str().unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_rejects_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(resolve(root, "../etc/passwd").is_err());
        assert!(resolve(root, "a/../../x").is_err());
        assert!(resolve(root, "/etc/passwd").is_err());
        assert_eq!(resolve(root, "a/../b.txt").unwrap(), root.join("b.txt"));
        assert_eq!(resolve(root, ".").unwrap(), root.to_path_buf());
    }

    #[tokio::test]
    async fn write_then_read() {
        let dir = tempfile::tempdir().unwrap();
        let w = WriteFile::new(dir.path().into());
        w.call(json!({"path": "notes/a.md", "content": "hi"})).await.unwrap();
        let r = ReadFile::new(dir.path().into());
        assert_eq!(r.call(json!({"path": "notes/a.md"})).await.unwrap(), "hi");
        let l = ListDir::new(dir.path().into());
        assert_eq!(l.call(json!({"path": "."})).await.unwrap(), "notes/");
    }
}
