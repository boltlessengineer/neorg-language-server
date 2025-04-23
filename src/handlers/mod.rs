use log::error;

use crate::session::Session;

use self::{
    notification::{handle_did_change, handle_did_close, handle_did_open},
    request::{
        handle_definition, handle_document_symbol, handle_references,
        handle_will_rename_files,
    },
};

mod notification;
mod request;

pub fn handle_req(session: &mut Session, req: lsp_server::Request) -> Option<lsp_server::Response> {
    error!("{}", req.method);
    match req.method.as_str() {
        // "textDocument/completion" => todo!(),
        "textDocument/documentSymbol" => Some(handle_document_symbol(session, req)),
        "textDocument/definition" => Some(handle_definition(session, req)),
        "textDocument/references" => Some(handle_references(session, req)),
        "workspace/willRenameFiles" => Some(handle_will_rename_files(session, req)),
        _ => None,
    }
}

pub fn handle_noti(
    session: &mut Session,
    noti: lsp_server::Notification,
) -> Option<lsp_server::Response> {
    match noti.method.as_str() {
        "textDocument/didOpen" => handle_did_open(session, noti.params),
        "textDocument/didChange" => handle_did_change(session, noti.params),
        "textDocument/didClose" => handle_did_close(session, noti.params),
        _ => (),
    };
    return None;
}
