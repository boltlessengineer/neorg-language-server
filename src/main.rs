mod document;
mod handlers;
mod neorg;
mod norg;
mod range;
mod session;
mod syntax;
mod tree_sitter;
mod workspace;

use std::fs::File;

use anyhow::Result;
use log::{error, warn};
use lsp_server::{Connection, Message};
use lsp_types::{
    CompletionOptions, FileOperationFilter, FileOperationPattern, FileOperationRegistrationOptions,
    InitializeParams, OneOf, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkDoneProgressOptions, WorkspaceFileOperationsServerCapabilities,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};
use neorg_dirman::workspace::Workspace;
use session::Session;

use crate::handlers::{handle_noti, handle_req};

fn main_loop(connection: Connection, mut session: Session) -> Result<()> {
    error!("Server Initialized!!");
    for msg in &connection.receiver {
        error!("connection received msg: {:?}", msg);
        // TODO: handle message asynchronously
        let resp = match msg {
            Message::Request(req) => handle_req(&mut session, req),
            Message::Response(_) => continue,
            Message::Notification(noti) => handle_noti(&mut session, noti),
        };
        if let Some(resp) = resp {
            connection.sender.send(Message::Response(resp))?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let log_file = File::options()
        .create(true)
        .append(true)
        .open("neorg.log")
        .expect("can't open log file");
    // structured_logger::Builder::with_level("WARN")
    //     .with_target_writer("*", structured_logger::json::new_writer(log_file))
    //     .init();
    let _ = simplelog::WriteLogger::init(log::LevelFilter::Warn, Default::default(), log_file);
    let (connection, iothreads) = Connection::stdio();
    let server_capabilities = ServerCapabilities {
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
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
                ..Default::default()
            }),
            file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                will_rename: Some(FileOperationRegistrationOptions {
                    filters: vec![
                        FileOperationFilter {
                            pattern: FileOperationPattern {
                                glob: "*.norg".to_string(),
                                matches: None,
                                options: None,
                            },
                            scheme: None,
                        },
                        FileOperationFilter {
                            pattern: FileOperationPattern {
                                glob: "**/*.norg".to_string(),
                                matches: None,
                                options: None,
                            },
                            scheme: None,
                        },
                    ],
                }),
                ..Default::default()
            }),
        }),
        ..Default::default()
    };
    let init_params = connection.initialize(serde_json::to_value(server_capabilities)?)?;
    let init_params: InitializeParams = serde_json::from_value(init_params)?;
    let workspace = init_params
        .root_uri
        .as_ref()
        .filter(|uri| uri.scheme() == "file")
        .and_then(|uri| uri.to_file_path().ok())
        .map(Workspace::from);
    let session = Session::with_workspace(workspace);
    main_loop(connection, session)?;
    iothreads.join()?;
    warn!("shut down");
    Ok(())
}
