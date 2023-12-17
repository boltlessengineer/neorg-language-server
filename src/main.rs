mod dirman;
mod document;
mod handlers;
mod norg;
mod tree_sitter;

use std::fs::File;

use anyhow::Result;
use log::{error, warn};
use lsp_server::{Connection, Message};
use lsp_types::{
    CompletionOptions, OneOf, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkDoneProgressOptions,
};
use structured_logger::{json::new_writer, Builder};

use crate::{
    document::init_doc_store,
    handlers::{handle_noti, handle_req},
    norg::init_norg_completion,
    tree_sitter::init_parser,
};

#[allow(unused_variables)]
fn main_loop(connection: Connection, params: serde_json::Value) -> Result<()> {
    error!("Server Initialized!!");
    for msg in &connection.receiver {
        // error!("connection received msg: {:?}", msg);
        let resp = match msg {
            Message::Request(req) => handle_req(req),
            Message::Response(_) => continue,
            Message::Notification(noti) => handle_noti(noti),
        };
        if let Some(resp) = resp {
            // error!("{resp:?}");
            connection.sender.send(Message::Response(resp))?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    init_norg_completion();
    init_parser();
    init_doc_store();
    let file = File::options()
        .create(true)
        .append(true)
        .open("neorg.log")
        .expect("can't open log file");
    let mut builder = Builder::with_level("WARN");
    builder = builder.with_target_writer("*", new_writer(file));
    builder.init();
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
        document_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })
    .unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params)?;
    iothreads.join()?;
    warn!("shut down");
    Ok(())
}
