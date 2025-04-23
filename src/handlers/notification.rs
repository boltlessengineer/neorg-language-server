use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

use crate::{document::Document, session::Session};

pub fn handle_did_open(session: &mut Session, params: serde_json::Value) {
    if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params) {
        let text_document = params.text_document;
        session.insert_document(text_document.uri, Document::new(&text_document.text)).unwrap();
        // TODO: handle error
    }
}

pub fn handle_did_change(session: &mut Session, params: serde_json::Value) {
    if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(params) {
        let text_document = params.text_document;
        let changes = params.content_changes;
        session.update_document(&text_document.uri, changes).unwrap();
        // TODO: handle error
    }
}

pub fn handle_did_close(session: &mut Session, params: serde_json::Value) {
    if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params) {
        let text_document = params.text_document;
        session.remove_document(&text_document.uri).unwrap();
    }
}
