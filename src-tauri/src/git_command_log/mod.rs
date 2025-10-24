use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;
use std::process::Output;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_LOG_ENTRIES: usize = 30;

#[derive(Debug, Clone, Serialize)]
pub struct GitCommandLogEntry {
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub success: bool,
    #[serde(default)]
    pub command_type: String,
}

pub struct GitCommandLog {
    entries: Mutex<VecDeque<GitCommandLogEntry>>,
}

impl GitCommandLog {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)),
        }
    }

    pub fn log_command(
        &self,
        command: &str,
        args: &[&str],
        working_dir: &str,
        output: &Output,
        duration: Duration,
    ) {
        self.log_command_with_type(command, args, working_dir, output, duration, "git");
    }

    pub fn log_command_with_type(
        &self,
        command: &str,
        args: &[&str],
        working_dir: &str,
        output: &Output,
        duration: Duration,
        command_type: &str,
    ) {
        let mut entries = self.entries.lock();

        if entries.len() >= MAX_LOG_ENTRIES {
            entries.pop_front();
        }

        let entry = GitCommandLogEntry {
            timestamp: Utc::now(),
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            working_dir: working_dir.to_string(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms: duration.as_millis() as u64,
            success: output.status.success(),
            command_type: command_type.to_string(),
        };

        entries.push_back(entry);
    }

    pub fn log_error(
        &self,
        command: &str,
        args: &[&str],
        working_dir: &str,
        error: &str,
        duration: Duration,
    ) {
        self.log_error_with_type(command, args, working_dir, error, duration, "git");
    }

    pub fn log_error_with_type(
        &self,
        command: &str,
        args: &[&str],
        working_dir: &str,
        error: &str,
        duration: Duration,
        command_type: &str,
    ) {
        let mut entries = self.entries.lock();

        if entries.len() >= MAX_LOG_ENTRIES {
            entries.pop_front();
        }

        let entry = GitCommandLogEntry {
            timestamp: Utc::now(),
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            working_dir: working_dir.to_string(),
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
            duration_ms: duration.as_millis() as u64,
            success: false,
            command_type: command_type.to_string(),
        };

        entries.push_back(entry);
    }

    pub fn log_editor_launch(
        &self,
        editor: &str,
        file_path: &str,
        line: Option<usize>,
        success: bool,
        error: Option<&str>,
        duration: Duration,
    ) {
        let mut entries = self.entries.lock();

        if entries.len() >= MAX_LOG_ENTRIES {
            entries.pop_front();
        }

        let mut args = vec![file_path.to_string()];
        if let Some(l) = line {
            args.push(format!("--line {}", l));
        }

        let entry = GitCommandLogEntry {
            timestamp: Utc::now(),
            command: format!("open-{}", editor),
            args,
            working_dir: ".".to_string(),
            exit_code: if success { Some(0) } else { Some(1) },
            stdout: if success {
                format!("Launched {} for {}", editor, file_path)
            } else {
                String::new()
            },
            stderr: error.unwrap_or("").to_string(),
            duration_ms: duration.as_millis() as u64,
            success,
            command_type: "editor".to_string(),
        };

        entries.push_back(entry);
    }

    pub fn log_request(
        &self,
        url: &str,
        success: bool,
        result: &str,
        details: &str,
        duration: Duration,
    ) {
        let mut entries = self.entries.lock();

        if entries.len() >= MAX_LOG_ENTRIES {
            entries.pop_front();
        }

        let entry = GitCommandLogEntry {
            timestamp: Utc::now(),
            command: url.to_string(),
            args: vec![result.to_string()],
            working_dir: ".".to_string(),
            exit_code: if success { Some(0) } else { Some(1) },
            stdout: if success {
                details.to_string()
            } else {
                String::new()
            },
            stderr: if success {
                String::new()
            } else {
                details.to_string()
            },
            duration_ms: duration.as_millis() as u64,
            success,
            command_type: "request".to_string(),
        };

        entries.push_back(entry);
    }

    pub fn get_entries(&self) -> Vec<GitCommandLogEntry> {
        self.entries.lock().iter().cloned().collect()
    }
}

lazy_static::lazy_static! {
    pub static ref GIT_COMMAND_LOG: Arc<GitCommandLog> = Arc::new(GitCommandLog::new());
}

pub fn run_git_command(working_dir: &str, args: &[&str]) -> std::io::Result<Output> {
    let start = Instant::now();

    let result = std::process::Command::new("git")
        .current_dir(working_dir)
        .args(args)
        .output();

    let duration = start.elapsed();

    match &result {
        Ok(output) => {
            GIT_COMMAND_LOG.log_command("git", args, working_dir, output, duration);
        }
        Err(e) => {
            GIT_COMMAND_LOG.log_error("git", args, working_dir, &e.to_string(), duration);
        }
    }

    result
}
