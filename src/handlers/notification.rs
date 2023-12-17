use log::error;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

use crate::document::{Document, DOC_STORE};

pub fn handle_did_open(params: serde_json::Value) {
    error!("did_open");
    if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params) {
        let text_document = params.text_document;
        let mut doc_store = DOC_STORE.get().unwrap().lock().unwrap();
        doc_store.insert(
            text_document.uri.to_string(),
            Document::new(text_document.text),
        );
    }
}

pub fn handle_did_change(params: serde_json::Value) {
    error!("did_change");
    if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(params) {
        let text_document = params.text_document;
        let changes = params.content_changes;
        let mut doc_store = DOC_STORE.get().unwrap().lock().unwrap();
        let uri = text_document.uri.to_string();
        let doc = doc_store.get_mut(&uri).expect("can't find document");
        error!("udpate text document");
        changes.iter().for_each(|change| {
            if let Some(range) = change.range {
                doc.change_range(range, &change.text);
            }
        });
        error!("doc:");
        error!("{}", doc.text.to_string());
        doc.update_tree();
        error!("tree:");
        error!("{}", doc.tree.root_node().to_sexp());
    }
}

pub fn handle_did_close(params: serde_json::Value) {
    error!("did_close");
    if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params) {
        let text_document = params.text_document;
        let mut doc_store = DOC_STORE.get().unwrap().lock().unwrap();
        let uri = text_document.uri.to_string();
        doc_store.remove(&uri);
    }
}
