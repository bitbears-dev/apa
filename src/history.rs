use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct HistoryRecord {
    pub timestamp: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<crate::planner::Plan>,
    pub exit_code: Option<i32>,
}

pub struct HistoryManager {
    path: PathBuf,
}

impl HistoryManager {
    pub fn new() -> Self {
        let path = if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "apa") {
            let dir = proj_dirs.data_dir();
            std::fs::create_dir_all(dir).ok();
            dir.join("history.jsonl")
        } else {
            PathBuf::from("history.jsonl")
        };

        Self { path }
    }

    pub fn append(&self, record: &HistoryRecord) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let json = serde_json::to_string(record)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }
}
