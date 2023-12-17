use lsp_types::InitializeParams;

#[derive(Debug)]
pub struct Config {
    pub init_params: InitializeParams,
}

impl Config {
    pub fn new(init_params: InitializeParams) -> Self {
        Self { init_params }
    }
}
