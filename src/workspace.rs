use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use log::error;
use neorg_dirman::workspace::{Workspace, WorkspaceManager};

pub static WS_MANAGER: OnceLock<Arc<Mutex<WorkspaceManager>>> = OnceLock::new();

pub fn init_worksapce(path: PathBuf) {
    let workspace = Workspace {
        name: path.display().to_string(),
        path,
    };
    error!("{workspace:?}");
    WS_MANAGER
        .set(Arc::new(Mutex::new(
            WorkspaceManager::from_single_workspace(workspace),
        )))
        .unwrap();
}
