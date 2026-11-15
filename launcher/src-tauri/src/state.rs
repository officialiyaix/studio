use crate::auth::AuthManager;
use crate::minecraft::install::Progress;
use crate::settings::Settings;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum RunPhase {
    Downloading { progress: Option<Progress> },
    Launching,
    Running { pid: u32, started_at: String },
    Error { message: String },
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub auth: AuthManager,
    pub runs: Mutex<HashMap<String, RunPhase>>,
    pub stoppers: Mutex<HashMap<String, oneshot::Sender<()>>>,
    pub news: Mutex<Vec<crate::news::NewsItem>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            settings: Mutex::new(Settings::load()),
            auth: AuthManager::new(),
            runs: Mutex::new(HashMap::new()),
            stoppers: Mutex::new(HashMap::new()),
            news: Mutex::new(Vec::new()),
        }
    }

    pub fn phase(&self, id: &str) -> Option<RunPhase> {
        self.runs.lock().unwrap().get(id).cloned()
    }

    pub fn is_busy(&self, id: &str) -> bool {
        matches!(self.phase(id), Some(RunPhase::Downloading { .. } | RunPhase::Launching | RunPhase::Running { .. }))
    }

    pub fn running_pid(&self, id: &str) -> Option<u32> {
        match self.phase(id) {
            Some(RunPhase::Running { pid, .. }) => Some(pid),
            _ => None,
        }
    }

    pub fn any_busy(&self) -> bool {
        self.runs.lock().unwrap().values().any(|p| !matches!(p, RunPhase::Error { .. }))
    }
}
