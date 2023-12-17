use crate::config::Config;

use self::{
    notification::{handle_did_change, handle_did_close, handle_did_open},
    request::{handle_completion, handle_definition, handle_document_symbol},
};

mod notification;
mod request;

pub fn handle_req(config: &Config, req: lsp_server::Request) -> Option<lsp_server::Response> {
    match req.method.as_str() {
        "textDocument/completion" => Some(handle_completion(req)),
        "textDocument/documentSymbol" => Some(handle_document_symbol(req)),
        "textDocument/definition" => Some(handle_definition(config, req)),
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
