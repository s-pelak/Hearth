//! SQLite storage for projects, threads and transcripts.

use crate::escalation::EscalationPolicy;
use crate::types::{Message, ModelRef};
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub instructions: String,
    pub folder: String,
    pub default_model: Option<ModelRef>,
    pub escalation: EscalationPolicy,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Working,
    NeedsYou,
    Review,
    Done,
    Failed,
}

impl ThreadStatus {
    fn as_str(self) -> &'static str {
        match self {
            ThreadStatus::Working => "working",
            ThreadStatus::NeedsYou => "needs_you",
            ThreadStatus::Review => "review",
            ThreadStatus::Done => "done",
            ThreadStatus::Failed => "failed",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "working" => ThreadStatus::Working,
            "needs_you" => ThreadStatus::NeedsYou,
            "review" => ThreadStatus::Review,
            "failed" => ThreadStatus::Failed,
            _ => ThreadStatus::Done,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Thread {
    pub id: String,
    /// None for a plain chat outside any project.
    pub project_id: Option<String>,
    pub title: String,
    pub status: ThreadStatus,
    pub model: ModelRef,
    pub updated_at: i64,
}

pub struct Store {
    db: Connection,
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        Self::init(db)
    }

    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(db: Connection) -> Result<Self> {
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS projects (
               id TEXT PRIMARY KEY, name TEXT NOT NULL, instructions TEXT NOT NULL DEFAULT '',
               folder TEXT NOT NULL, default_model TEXT, escalation TEXT NOT NULL, created_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS threads (
               id TEXT PRIMARY KEY, project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
               title TEXT NOT NULL, status TEXT NOT NULL, model TEXT NOT NULL,
               created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS messages (
               thread_id TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
               seq INTEGER NOT NULL, body TEXT NOT NULL, created_at INTEGER NOT NULL,
               PRIMARY KEY (thread_id, seq));
             CREATE TABLE IF NOT EXISTS usage (
               thread_id TEXT NOT NULL, provider TEXT NOT NULL, model TEXT NOT NULL,
               input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL, at INTEGER NOT NULL);",
        )?;
        Ok(Self { db })
    }

    pub fn create_project(&self, name: &str, folder: &str) -> Result<Project> {
        let p = Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            instructions: String::new(),
            folder: folder.into(),
            default_model: None,
            escalation: EscalationPolicy::default(),
        };
        self.save_project(&p)?;
        Ok(p)
    }

    pub fn save_project(&self, p: &Project) -> Result<()> {
        self.db.execute(
            "INSERT INTO projects (id, name, instructions, folder, default_model, escalation, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET name=?2, instructions=?3, folder=?4, default_model=?5, escalation=?6",
            params![
                p.id,
                p.name,
                p.instructions,
                p.folder,
                p.default_model.as_ref().map(serde_json::to_string).transpose()?,
                serde_json::to_string(&p.escalation)?,
                now()
            ],
        )?;
        Ok(())
    }

    pub fn projects(&self) -> Result<Vec<Project>> {
        let mut st = self.db.prepare("SELECT id, name, instructions, folder, default_model, escalation FROM projects ORDER BY created_at")?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, Option<String>>(4)?, r.get::<_, String>(5)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, name, instructions, folder, dm, esc) = row?;
            out.push(Project {
                id,
                name,
                instructions,
                folder,
                default_model: dm.map(|s| serde_json::from_str(&s)).transpose()?,
                escalation: serde_json::from_str(&esc).unwrap_or_default(),
            });
        }
        Ok(out)
    }

    pub fn create_thread(&self, project_id: Option<&str>, title: &str, model: &ModelRef) -> Result<Thread> {
        let t = Thread {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.map(String::from),
            title: title.into(),
            status: ThreadStatus::Working,
            model: model.clone(),
            updated_at: now(),
        };
        self.db.execute(
            "INSERT INTO threads (id, project_id, title, status, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![t.id, t.project_id, t.title, t.status.as_str(), serde_json::to_string(&t.model)?, t.updated_at],
        )?;
        Ok(t)
    }

    pub fn threads(&self, project_id: Option<&str>) -> Result<Vec<Thread>> {
        let mut st = self.db.prepare(
            "SELECT id, project_id, title, status, model, updated_at FROM threads
             WHERE project_id IS ?1 ORDER BY updated_at DESC",
        )?;
        let rows = st.query_map(params![project_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, i64>(5)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, project_id, title, status, model, updated_at) = row?;
            out.push(Thread { id, project_id, title, status: ThreadStatus::parse(&status), model: serde_json::from_str(&model)?, updated_at });
        }
        Ok(out)
    }

    pub fn set_status(&self, thread_id: &str, status: ThreadStatus) -> Result<()> {
        self.db.execute("UPDATE threads SET status=?2, updated_at=?3 WHERE id=?1", params![thread_id, status.as_str(), now()])?;
        Ok(())
    }

    pub fn append_message(&self, thread_id: &str, m: &Message) -> Result<()> {
        let seq: i64 = self
            .db
            .query_row("SELECT COALESCE(MAX(seq), -1) + 1 FROM messages WHERE thread_id=?1", params![thread_id], |r| r.get(0))?;
        self.db.execute(
            "INSERT INTO messages (thread_id, seq, body, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![thread_id, seq, serde_json::to_string(m)?, now()],
        )?;
        self.db.execute("UPDATE threads SET updated_at=?2 WHERE id=?1", params![thread_id, now()])?;
        Ok(())
    }

    pub fn messages(&self, thread_id: &str) -> Result<Vec<Message>> {
        let mut st = self.db.prepare("SELECT body FROM messages WHERE thread_id=?1 ORDER BY seq")?;
        let rows = st.query_map(params![thread_id], |r| r.get::<_, String>(0))?;
        rows.map(|b| Ok(serde_json::from_str(&b?)?)).collect()
    }

    pub fn record_usage(&self, thread_id: &str, model: &ModelRef, input: u64, output: u64) -> Result<()> {
        self.db.execute(
            "INSERT INTO usage (thread_id, provider, model, input_tokens, output_tokens, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![thread_id, model.provider, model.model, input as i64, output as i64, now()],
        )?;
        Ok(())
    }

    pub fn thread(&self, id: &str) -> Result<Option<Thread>> {
        Ok(self.threads_all()?.into_iter().find(|t| t.id == id))
    }

    fn threads_all(&self) -> Result<Vec<Thread>> {
        let ids: Vec<Option<String>> = {
            let mut st = self.db.prepare("SELECT DISTINCT project_id FROM threads")?;
            st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?
        };
        let mut out = Vec::new();
        for pid in ids {
            out.extend(self.threads(pid.as_deref())?);
        }
        Ok(out)
    }

    pub fn project(&self, id: &str) -> Result<Option<Project>> {
        Ok(self.projects()?.into_iter().find(|p| p.id == id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let s = Store::in_memory().unwrap();
        let p = s.create_project("Benchmarks", "/tmp/b").unwrap();
        assert!(!s.projects().unwrap()[0].escalation.enabled);
        let m = ModelRef::new("hearth-box", "qwen3.5-9b");
        let t = s.create_thread(Some(&p.id), "Test", &m).unwrap();
        s.append_message(&t.id, &Message::user("hi")).unwrap();
        s.append_message(&t.id, &Message::assistant("hello")).unwrap();
        assert_eq!(s.messages(&t.id).unwrap().len(), 2);
        s.set_status(&t.id, ThreadStatus::Review).unwrap();
        assert_eq!(s.thread(&t.id).unwrap().unwrap().status, ThreadStatus::Review);
        let chat = s.create_thread(None, "Chat", &m).unwrap();
        assert_eq!(s.threads(None).unwrap()[0].id, chat.id);
    }
}
