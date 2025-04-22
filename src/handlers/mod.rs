use log::error;

use crate::state::State;

use self::{
    notification::{handle_did_change, handle_did_close, handle_did_open},
    request::{
        handle_completion, handle_definition, handle_document_symbol, handle_references,
        handle_will_rename_files,
    },
};

mod notification;
mod request;

pub fn handle_req(state: &State, req: lsp_server::Request) -> Option<lsp_server::Response> {
    error!("{}", req.method);
    match req.method.as_str() {
        "textDocument/completion" => Some(handle_completion(req)),
        "textDocument/documentSymbol" => Some(handle_document_symbol(req)),
        "textDocument/definition" => Some(handle_definition(state, req)),
        "textDocument/references" => Some(handle_references(state, req)),
        "workspace/willRenameFiles" => Some(handle_will_rename_files(state, req)),
        _ => None,
    }
}

pub fn handle_noti(noti: lsp_server::Notification) -> Option<lsp_server::Response> {
    match noti.method.as_str() {
        "textDocument/didOpen" => handle_did_open(noti.params),
        "textDocument/didChange" => handle_did_change(noti.params),
        "textDocument/didClose" => handle_did_close(noti.params),
        _ => (),
    };
    return None;
}
