use std::path::Path;

use log::{debug, error};
use lsp_server::{ErrorCode, Response};
use lsp_types::{
    CompletionItem, CompletionList, CompletionParams, DocumentSymbol, DocumentSymbolParams,
    Documentation, GotoDefinitionParams, GotoDefinitionResponse, InsertTextFormat, Location,
    ReferenceParams, Url,
};
use walkdir::WalkDir;

use crate::{
    config::Config,
    document::{Document, DOC_STORE},
    norg::NORG_BLOCKS,
    tree_sitter::_Range,
};

pub fn handle_completion(req: lsp_server::Request) -> Response {
    let params: CompletionParams = serde_json::from_value(req.params).unwrap();
    let uri = params.text_document_position.text_document.uri.to_string();
    let pos = params.text_document_position.position;
    error!("pos: {pos:?}");
    let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
    let doc = doc_store.get(&uri).unwrap();
    let node = doc.get_node_from_range(pos).expect("can't get node");
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
            let range = node.range().as_lsp_range();
            let sym = DocumentSymbol {
                name,
                detail: Some(node.utf8_text(text).unwrap().to_string()),
                kind: lsp_types::SymbolKind::STRUCT,
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

    use super::*;

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

pub fn handle_document_symbol(req: lsp_server::Request) -> Response {
    error!("document symbol");
    let params: DocumentSymbolParams = serde_json::from_value(req.params).unwrap();
    let uri = params.text_document.uri.to_string();
    let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
    let doc = doc_store.get(&uri).unwrap();
    let doc_text = doc.text.to_string();
    let symbols = tree_to_symbols(&mut doc.tree.walk(), doc_text.as_bytes());
    return Response::new_ok(req.id, serde_json::to_value(symbols).unwrap());
}

pub fn handle_definition(req: lsp_server::Request) -> Response {
    error!("goto definition");
    let params: GotoDefinitionParams = serde_json::from_value(req.params).unwrap();
    let req_uri = params.text_document_position_params.text_document.uri;
    let req_pos = params.text_document_position_params.position;
    let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
    let doc = doc_store.get(&req_uri.to_string()).unwrap();
    if let Some(link) = doc.get_link_from_pos(req_pos) {
        debug!("{link:?}");
        match link.as_uri(&req_uri) {
            Ok(link_uri) => {
                let definitions = GotoDefinitionResponse::Scalar(lsp_types::Location {
                    uri: link_uri,
                    range: Default::default(),
                });
                return Response::new_ok(req.id, serde_json::to_value(definitions).unwrap());
            }
            Err(_) => {
                return Response::new_err(
                    req.id,
                    ErrorCode::RequestFailed as i32,
                    "workspace link is not supported yet".to_string(),
                );
            }
        }
    }
    return Response::new_err(
        req.id,
        lsp_server::ErrorCode::RequestFailed as i32,
        "can't find link in request position".to_string(),
    );
}

pub fn handle_references(config: &Config, req: lsp_server::Request) -> Response {
    let params: ReferenceParams = serde_json::from_value(req.params).unwrap();
    let req_uri = params.text_document_position.text_document.uri;
    let req_pos = params.text_document_position.position;
    let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
    let doc = doc_store.get(&req_uri.to_string()).unwrap();
    if let Some(link) = doc.get_link_from_pos(req_pos) {
        // walk through directory and parse all documents, capture links
        // filter link by destination
        error!("{link:?}");
        // HACK: use virtual workspace instead
        #[allow(deprecated)]
        let root_path = &config.init_params.root_path.as_ref().unwrap();
        if let Ok(req_link_uri) = link.as_uri(&req_uri) {
            let references: Vec<Location> = list_references_from_uri(req_link_uri, &root_path)
                .into_iter()
                .map(|(origin, range)| Location { uri: origin, range })
                .collect();
            return Response::new_ok(req.id, serde_json::to_value(references).unwrap());
        } else {
            return Response::new_err(
                req.id,
                lsp_server::ErrorCode::RequestFailed as i32,
                "workspace links are not implemented yet".to_string(),
            );
        }
    }
    return Response::new_err(
        req.id,
        lsp_server::ErrorCode::InvalidRequest as i32,
        "can't find link in request position".to_string(),
    );
}

fn list_references_from_uri<P: AsRef<Path>>(uri: Url, root: P) -> Vec<(Url, lsp_types::Range)> {
    // TODO: generalize with crate::workspace::Workspace::iter_documents()
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? == "norg" {
                let url = Url::from_file_path(path.to_owned()).ok()?;
                return Document::from_path(path)
                    .ok()
                    .map(|d| (url, d));
            }
            None
        })
        .flat_map(|(path, d)| d.links.into_iter().map(move |l| (path.clone(), l)))
        .filter(|(origin, link)| {
            if let Ok(link_uri) = link.as_uri(origin) {
                uri == link_uri
            } else {
                false
            }
        })
        .map(|(path, l)| (path, l.range.as_lsp_range()))
        .collect()
}

#[cfg(test)]
mod test_request {
    use std::path::Path;

    use super::*;

    #[test]
    fn list_references() {
        structured_logger::Builder::with_level("debug").init();
        let path = Path::new("test/index.norg");
        let current_dir = std::env::current_dir().expect("failed to get current dir");
        list_references_from_uri(
            Url::from_file_path(current_dir.join(path)).unwrap(),
            current_dir.join("./test"),
        );
    }
}
