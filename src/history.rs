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

pub fn append_to_shell_history(command: &str) -> anyhow::Result<()> {
    // If running under a shell wrapper, write to the hook file instead of manipulating history files directly.
    if let Ok(hook_file) = std::env::var("APA_HISTORY_HOOK_FILE") {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&hook_file)?;
        write!(file, "{}", command)?;
        return Ok(());
    }

    let shell = std::env::var("SHELL").unwrap_or_default();
    let is_zsh = shell.ends_with("zsh");
    let is_bash = shell.ends_with("bash");

    if !is_zsh && !is_bash {
        return Ok(());
    }

    let histfile = match std::env::var("HISTFILE") {
        Ok(val) => PathBuf::from(val),
        Err(_) => {
            if let Some(base_dirs) = directories::BaseDirs::new() {
                if is_zsh {
                    base_dirs.home_dir().join(".zsh_history")
                } else {
                    base_dirs.home_dir().join(".bash_history")
                }
            } else {
                return Ok(());
            }
        }
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&histfile)?;

    if is_zsh {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        writeln!(file, ": {}:0;{}", now, command)?;
    } else {
        writeln!(file, "{}", command)?;
    }

    Ok(())
}
