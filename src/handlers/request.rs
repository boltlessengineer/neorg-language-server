use log::{debug, error};
use lsp_server::{ErrorCode, Response};
use lsp_types::{
    CompletionItem, CompletionList, CompletionParams, DocumentSymbol, DocumentSymbolParams,
    Documentation, GotoDefinitionParams, GotoDefinitionResponse, InsertTextFormat, Url,
};

use crate::{
    config::Config,
    document::DOC_STORE,
    norg::NORG_BLOCKS,
    tree_sitter::{ts_to_lsp_range, LinkDestination, LinkRoot},
};

pub fn handle_completion(req: lsp_server::Request) -> Response {
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

pub fn handle_definition(config: &Config, req: lsp_server::Request) -> Response {
    error!("goto definition");
    let params: GotoDefinitionParams = serde_json::from_value(req.params).unwrap();
    let req_uri = params.text_document_position_params.text_document.uri;
    let req_pos = params.text_document_position_params.position;
    let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
    let doc = doc_store.get(&req_uri.to_string()).unwrap();
    if let Some(link) = doc.get_link_from_pos(req_pos.into()) {
        debug!("{link:?}");
        let definitions = match link.destination {
            LinkDestination::Uri(uri) => GotoDefinitionResponse::Scalar(lsp_types::Location {
                uri: Url::parse(&uri).expect("invalid url"),
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                    end: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                },
            }),
            LinkDestination::NorgFile {
                root: LinkRoot::Workspace(_workspace),
                path: _path,
            } => {
                // GotoDefinitionResponse::Array(vec![])
                return Response::new_err(
                    req.id,
                    ErrorCode::RequestFailed as i32,
                    "workspace link is not supported yet".to_string(),
                );
            }
            LinkDestination::NorgFile { root, path } => {
                let path = if path.ends_with(".norg") {
                    path
                } else {
                    path + ".norg"
                };
                let uri = match root {
                    LinkRoot::None => req_uri.join(&path).unwrap(),
                    LinkRoot::Root => Url::parse(&format!("file:///{}", &path)).unwrap(),
                    LinkRoot::Workspace(_) | LinkRoot::Current => {
                        return Response::new_err(
                            req.id,
                            ErrorCode::RequestFailed as i32,
                            "workspace link is not supported yet".to_string(),
                        );
                    }
                    #[allow(unreachable_patterns)]
                    LinkRoot::Current => {
                        let mut uri = config.init_params.root_uri.as_ref().unwrap().clone();
                        uri.path_segments_mut().unwrap().push(&path);
                        uri
                    }
                };
                GotoDefinitionResponse::Scalar(lsp_types::Location {
                    uri,
                    range: lsp_types::Range {
                        start: lsp_types::Position {
                            line: 0,
                            character: 0,
                        },
                        end: lsp_types::Position {
                            line: 0,
                            character: 0,
                        },
                    },
                })
            }
        };
        return Response::new_ok(req.id, serde_json::to_value(definitions).unwrap());
    }
    return Response::new_err(
        req.id,
        lsp_server::ErrorCode::RequestFailed as i32,
        "can't find link in request position".to_string(),
    );
}
