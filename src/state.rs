use neorg_dirman::workspace::Workspace;

#[derive(Debug)]
pub struct State {
    pub workspace: Option<Workspace>,
}

impl State {
    pub fn new(workspace: Option<Workspace>) -> Self {
        Self { workspace }
    }
}
