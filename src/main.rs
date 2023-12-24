mod config;
mod document;
mod handlers;
mod norg;
mod tree_sitter;
mod workspace;

use std::{fs::File, path::PathBuf};

use anyhow::Result;
use log::{error, warn};
use lsp_server::{Connection, Message};
use lsp_types::{
    CompletionOptions, InitializeParams, OneOf, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkDoneProgressOptions,
};
use structured_logger::{json::new_writer, Builder};

use crate::{
    config::Config,
    document::init_doc_store,
    handlers::{handle_noti, handle_req},
    norg::init_norg_completion,
    workspace::init_worksapce,
};

fn main_loop(connection: Connection, config: &Config) -> Result<()> {
    error!("Server Initialized!!");
    for msg in &connection.receiver {
        error!("connection received msg: {:?}", msg);
        let resp = match msg {
            Message::Request(req) => handle_req(config, req),
            Message::Response(_) => continue,
            Message::Notification(noti) => handle_noti(noti),
        };
        if let Some(resp) = resp {
            connection.sender.send(Message::Response(resp))?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    init_norg_completion();
    init_doc_store();
    // TODO: init_workspace();
    let log_file = File::options()
        .create(true)
        .append(true)
        .open("neorg.log")
        .expect("can't open log file");
    Builder::with_level("WARN")
        .with_target_writer("*", new_writer(log_file))
        .init();
    let (connection, iothreads) = Connection::stdio();
    let server_capabilities = serde_json::to_value(ServerCapabilities {
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![]),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
            completion_item: None,
        }),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })
    .unwrap();
    let init_params = connection.initialize(server_capabilities)?;
    let init_params: InitializeParams = serde_json::from_value(init_params)?;
    let config = Config::new(init_params);
    init_worksapce(PathBuf::from(
        config
            .init_params
            .root_uri
            .as_ref()
            .unwrap()
            .to_file_path()
            .unwrap(),
    ));
    main_loop(connection, &config)?;
    iothreads.join()?;
    warn!("shut down");
    Ok(())
}
