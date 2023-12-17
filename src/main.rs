mod dirman;
mod document;
mod norg;
mod tree_sitter;

use std::fs::File;

use anyhow::Result;
use log::{error, warn};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response, ResponseError};
use lsp_types::{
    CompletionItem, CompletionList, CompletionOptions, CompletionParams,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbol, DocumentSymbolParams, Documentation, GotoDefinitionParams,
    GotoDefinitionResponse, InsertTextFormat, OneOf, ServerCapabilities, SymbolKind,
    TextDocumentSyncCapability, TextDocumentSyncKind, WorkDoneProgressOptions,
};
use structured_logger::{json::new_writer, Builder};

use crate::{
    document::{init_doc_store, Document, DOC_STORE},
    norg::{init_norg_completion, NORG_BLOCKS},
    tree_sitter::{init_parser, ts_to_lsp_range},
};

struct NorgResponse {
    result: Option<serde_json::Value>,
    error: Option<lsp_server::ResponseError>,
}

fn handle_completion(req: Request) -> Response {
    let params: CompletionParams = serde_json::from_value(req.params).unwrap();
    let uri = params.text_document_position.text_document.uri.to_string();
    let pos = params.text_document_position.position;
    error!("pos: {pos:?}");
    let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
    let doc = doc_store.get(&uri).unwrap();
    error!("{}", doc.text.to_string());
    let node = doc.get_node_from_range(pos.into()).expect("can't get node");
    error!("{}", node.to_sexp());
    let list = CompletionList {
        is_incomplete: true,
        items: NORG_BLOCKS
            .get()
            .unwrap()
            .iter()
            .filter(|item| {
                // HACK: find better way (maybe with query?)
                let mut node = node;
                let skip = vec!["paragraph", "para_break"];
                if item.valid_parents.len() == 0 {
                    return true;
                }
                while skip.contains(&node.kind()) {
                    if let Some(parent) = node.parent() {
                        node = parent
                    } else {
                        break;
                    }
                }
                return item
                    .valid_parents
                    .contains(&node.kind().to_string().clone());
            })
            .map(|item| CompletionItem {
                label: item.name.clone(),
                kind: Some(item.kind),
                documentation: Some(Documentation::String(item.desc.clone())),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                insert_text: Some(item.text.clone()),
                ..Default::default()
            })
            .collect(),
    };
    return Response::new_ok(req.id, serde_json::to_value(list).unwrap());
}

// TODO: move to Document
#[allow(deprecated)]
fn tree_to_symbols(cursor: &mut ::tree_sitter::TreeCursor, text: &[u8]) -> Vec<DocumentSymbol> {
    let node = cursor.node();
    let mut symbols: Vec<DocumentSymbol> = vec![];
    if node.is_named() {
        let name = match node.kind() {
            "document" => {
                cursor.goto_first_child();
                return tree_to_symbols(cursor, text);
            }
            "heading" => {
                // TODO: return range that can used for `name` and `selection_range`
                // TODO: also should return the symbol type
                // TODO: if field `title` is empty (slide/indent segments,) create
                // title with first non-empty line
                let _title = node.child_by_field_name("title");
                Some("heading")
            }
            // TODO: should we return more than heading..?
            // they are one of symbols according to spec, but user would prefer only
            // linkable symbols
            "standard_ranged_tag" => Some("ranged tag"),
            _ => None,
        };
        if let Some(name) = name {
            let name = name.to_string();
            let range = ts_to_lsp_range(node.range());
            let sym = DocumentSymbol {
                name,
                detail: Some(node.utf8_text(text).unwrap().to_string()),
                kind: SymbolKind::STRUCT,
                tags: None,
                range,
                selection_range: range,
                children: if cursor.goto_first_child() {
                    let children = tree_to_symbols(cursor, text);
                    if children.len() > 0 {
                        Some(children)
                    } else {
                        None
                    }
                } else {
                    None
                },
                deprecated: None,
            };
            symbols.push(sym);
        }
    }
    if cursor.goto_next_sibling() {
        symbols.append(&mut tree_to_symbols(cursor, text));
    } else {
        cursor.goto_parent();
    }
    return symbols;
}

#[cfg(test)]
mod test {
    use lsp_types::{DocumentSymbol, Position, Range, SymbolKind};
    use tree_sitter::Parser;

    use crate::tree_to_symbols;

    #[allow(deprecated)]
    #[test]
    fn test_tree_to_sym() {
        let doc = r#"|example
* h1eading
helo
|end
* h2eading
"#;
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_norg3::language())
            .expect("could not load norg parser");
        let tree = parser.parse(doc, None).expect("get tree");
        let mut cursor = tree.walk();
        let symbols = tree_to_symbols(&mut cursor, doc.as_bytes());
        assert_eq!(
            vec![
                DocumentSymbol {
                    name: "ranged tag".to_string(),
                    detail: Some("|example\n* h1eading\nhelo\n|end\n".to_string()),
                    kind: SymbolKind::STRUCT,
                    tags: None,
                    deprecated: None,
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 4,
                            character: 0,
                        },
                    },
                    selection_range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 4,
                            character: 0,
                        },
                    },
                    children: Some(vec![DocumentSymbol {
                        name: "heading".to_string(),
                        detail: Some("* h1eading\nhelo\n".to_string()),
                        kind: SymbolKind::STRUCT,
                        tags: None,
                        deprecated: None,
                        range: Range {
                            start: Position {
                                line: 1,
                                character: 0,
                            },
                            end: Position {
                                line: 3,
                                character: 0,
                            },
                        },
                        selection_range: Range {
                            start: Position {
                                line: 1,
                                character: 0,
                            },
                            end: Position {
                                line: 3,
                                character: 0,
                            },
                        },
                        children: None,
                    }]),
                },
                DocumentSymbol {
                    name: "heading".to_string(),
                    detail: Some("* h2eading\n".to_string()),
                    kind: SymbolKind::STRUCT,
                    tags: None,
                    deprecated: None,
                    range: Range {
                        start: Position {
                            line: 4,
                            character: 0,
                        },
                        end: Position {
                            line: 5,
                            character: 0,
                        },
                    },
                    selection_range: Range {
                        start: Position {
                            line: 4,
                            character: 0,
                        },
                        end: Position {
                            line: 5,
                            character: 0,
                        },
                    },
                    children: None,
                },
            ],
            symbols
        );
    }
}

fn handle_document_symbol(req: Request) -> Response {
    error!("document symbol");
    let params: DocumentSymbolParams = serde_json::from_value(req.params).unwrap();
    let uri = params.text_document.uri.to_string();
    let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
    let doc = doc_store.get(&uri).unwrap();
    let doc_text = doc.text.to_string();
    let symbols = tree_to_symbols(&mut doc.tree.walk(), doc_text.as_bytes());
    return Response::new_ok(req.id, serde_json::to_value(symbols).unwrap());
}

fn handle_definition(req: Request) -> Response {
    error!("goto definition");
    let params: GotoDefinitionParams = serde_json::from_value(req.params).unwrap();
    let req_uri = params
        .text_document_position_params
        .text_document
        .uri
        .to_string();
    let req_pos = params.text_document_position_params.position;
    let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
    let doc = doc_store.get(&req_uri).unwrap();
    if let Some(link) = doc.get_link_from_pos(req_pos.into()) {
        // TODO: get definition from parsed link
        error!("{link:?}");
        // let definitions = GotoDefinitionResponse::Scalar(lsp_types::Location {
        //     uri: params.text_document_position_params.text_document.uri,
        //     range: lsp_types::Range {
        //         start: lsp_types::Position { line: 0, character: 0 },
        //         end: lsp_types::Position { line: 0, character: 2 },
        //     },
        // });
        let definitions = GotoDefinitionResponse::Array(vec![]);
        return Response::new_ok(req.id, serde_json::to_value(definitions).unwrap());
    }
    return Response::new_err(
        req.id,
        ErrorCode::RequestFailed as i32,
        "can't find link in request position".to_string(),
    );
}

fn handle_req(req: Request) -> Option<Response> {
    match req.method.as_str() {
        "textDocument/completion" => Some(handle_completion(req)),
        "textDocument/documentSymbol" => Some(handle_document_symbol(req)),
        "textDocument/definition" => Some(handle_definition(req)),
        _ => None,
    }
}

fn handle_did_open(params: serde_json::Value) {
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

fn handle_did_change(params: serde_json::Value) {
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

fn handle_did_close(params: serde_json::Value) {
    error!("did_close");
    if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params) {
        let text_document = params.text_document;
        let mut doc_store = DOC_STORE.get().unwrap().lock().unwrap();
        let uri = text_document.uri.to_string();
        doc_store.remove(&uri);
    }
}

fn handle_noti(noti: Notification) -> Option<Response> {
    match noti.method.as_str() {
        "textDocument/didOpen" => handle_did_open(noti.params),
        "textDocument/didChange" => handle_did_change(noti.params),
        "textDocument/didClose" => handle_did_close(noti.params),
        _ => (),
    };
    return None;
}

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
