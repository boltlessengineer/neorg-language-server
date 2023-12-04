use std::fs::File;

use anyhow::Result;
use log::{error, warn};
use lsp_server::{Connection, Message, Request, RequestId, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionOptions, CompletionParams,
    ServerCapabilities, WorkDoneProgressOptions,
};
use structured_logger::{json::new_writer, Builder};

fn handle_completion(params: serde_json::Value) -> Option<serde_json::Value> {
    let params: CompletionParams = serde_json::from_value(params).unwrap();
    let cmp_items = vec!["code", "macro", "table"];
    let list = CompletionList {
        is_incomplete: true,
        items: cmp_items
            .iter()
            .map(|x| CompletionItem {
                kind: Some(CompletionItemKind::TEXT),
                label: x.to_string(),
                ..Default::default()
            })
            .collect(),
    };
    return Some(serde_json::to_value(list).unwrap());
    // return None;
}

fn handle_req(req: Request) -> Option<Response> {
    let result = match req.method.as_str() {
        "textDocument/completion" => handle_completion(req.params),
        _ => None,
    };
    return Some(Response {
        id: req.id,
        result,
        error: None,
    });
}

fn main_loop(connection: Connection, params: serde_json::Value) -> Result<()> {
    for msg in &connection.receiver {
        error!("connection received msg: {:?}", msg);
        let resp = match msg {
            Message::Request(req) => handle_req(req),
            Message::Response(_) => continue,
            Message::Notification(_) => continue,
        };
        if let Some(resp) = resp {
            error!("{resp:?}");
            connection.sender.send(Message::Response(resp))?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let file = File::options()
        .create(true)
        .append(true)
        .open("neorg.log")
        .expect("can't open log file");
    let mut builder = Builder::with_level("ERROR");
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
        ..Default::default()
    })
    .unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;
    error!("hello world!");
    main_loop(connection, initialization_params)?;
    iothreads.join()?;
    warn!("shut down");
    Ok(())
}
