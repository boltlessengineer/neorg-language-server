use log::error;
use lsp_server::Response;
use lsp_types::{
    DocumentSymbol, DocumentSymbolParams,
    GotoDefinitionParams, GotoDefinitionResponse, ReferenceParams,
};

use crate::{neorg, session::Session, tree_sitter::ToLspRange};

fn tree_to_symbols(cursor: &mut ::tree_sitter::TreeCursor, text: &[u8]) -> Vec<DocumentSymbol> {
    let node = cursor.node();
    let mut symbols: Vec<DocumentSymbol> = vec![];
    if node.is_named() {
        match node.kind() {
            "document" => {
                cursor.goto_first_child();
                return tree_to_symbols(cursor, text);
            }
            "section" => {
                // TODO: return range that can used for `name` and `selection_range`
                // also should return the symbol type
                let title_node = node
                    .child_by_field_name("heading")
                    .unwrap()
                    .child_by_field_name("title")
                    .unwrap();
                let title = title_node.utf8_text(text).unwrap();
                Some(title.to_string())
            }
            // TODO: add more symbols. (if final syntax has more than headings)
            _ => None,
        }
        .map(|name| {
            let range = node.range().to_lsp_range();
            #[allow(deprecated)]
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
        });
    }
    if cursor.goto_next_sibling() {
        symbols.append(&mut tree_to_symbols(cursor, text));
    } else {
        cursor.goto_parent();
    }
    return symbols;
}

pub fn handle_document_symbol(session: &Session, req: lsp_server::Request) -> Response {
    error!("document symbol");
    let params: DocumentSymbolParams = serde_json::from_value(req.params).unwrap();
    let uri = params.text_document.uri;
    let doc = session.get_document(&uri).unwrap();
    let doc_text = doc.text.to_string();
    let symbols = tree_to_symbols(&mut doc.tree.walk(), doc_text.as_bytes());
    return Response::new_ok(req.id, symbols);
}

pub fn handle_definition(session: &Session, req: lsp_server::Request) -> Response {
    error!("goto definition");
    let params: GotoDefinitionParams = serde_json::from_value(req.params).unwrap();
    let req_uri = params.text_document_position_params.text_document.uri;
    let req_pos = params.text_document_position_params.position;
    if let Some(locs) = neorg::definition(session, req_uri, req_pos) {
        if locs.len() == 1 {
            Response::new_ok(req.id, GotoDefinitionResponse::Scalar(locs[0].clone()))
        } else {
            Response::new_ok(req.id, GotoDefinitionResponse::Array(locs))
        }
    } else {
        Response::new_err(
            req.id,
            lsp_server::ErrorCode::RequestFailed as i32,
            "can't find link in request position".to_string(),
        )
    }
}

pub fn handle_references(session: &Session, req: lsp_server::Request) -> Response {
    if session.workspace.is_none() {
        todo!()
    }
    let params: ReferenceParams = serde_json::from_value(req.params).unwrap();
    let req_uri = params.text_document_position.text_document.uri;
    let req_pos = params.text_document_position.position;
    match neorg::references(session, req_uri, req_pos) {
        Some(references) => Response::new_ok(req.id, references),
        None => Response::new_err(
            req.id,
            lsp_server::ErrorCode::RequestFailed as i32,
            "reference not found".to_string(),
        ),
    }
}

// pub fn handle_will_rename_files(session: &Session, req: lsp_server::Request) -> Response {
//     error!("willrename");
//     let Some(ref workspace) = session.workspace else {
//         // return Response::new_err(req.id, code, message)
//         todo!()
//     };
//     let params: RenameFilesParams = serde_json::from_value(req.params).unwrap();
//     let mut changes: HashMap<Url, Vec<lsp_types::TextEdit>> = HashMap::new();
//     params.files.iter().for_each(|file_rename| {
//         iter_links(workspace)
//             .filter(
//                 |(url, link)| match workspace.resolve_link_location(url, &link.destination) {
//                     Some(loc) => loc.uri.to_string() == file_rename.old_uri,
//                     None => false,
//                 },
//             )
//             .for_each(|(link_origin, link)| {
//                 let mut new_dest = link.destination;
//                 new_dest.update_uri(&file_rename.new_uri).unwrap();
//                 let textedit =
//                     lsp_types::TextEdit::new(link.dest_range.to_lsp_range(), new_dest.to_string());
//                 changes
//                     .entry(link_origin)
//                     .and_modify(|v| v.push(textedit.clone()))
//                     .or_insert(vec![textedit]);
//             });
//     });
//     let workspace_edit = lsp_types::WorkspaceEdit {
//         changes: Some(changes),
//         ..Default::default()
//     };
//     Response::new_ok(req.id, workspace_edit)
// }

#[cfg(test)]
mod test {
    use lsp_types::{Position, Range, SymbolKind};
    use tree_sitter::Parser;

    use super::*;

    // macro_rules! url {
    //     ($path:expr) => {
    //         Url::from_file_path(
    //             std::env::current_dir()
    //                 .expect("failed to get current dir")
    //                 .join($path),
    //         )
    //         .unwrap()
    //     };
    // }

    macro_rules! range {
        ($sl:expr, $sc:expr, $el:expr, $ec:expr) => {
            Range::new(Position::new($sl, $sc), Position::new($el, $ec))
        };
    }

    #[test]
    fn document_symbols() {
        let doc = r#"
        * h1eading
        helo
        * h2eading
        ** h2-1eading
        "#;
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_norg::LANGUAGE.into())
            .expect("could not load norg parser");
        let tree = parser.parse(doc, None).expect("get tree");
        let mut cursor = tree.walk();
        let symbols = tree_to_symbols(&mut cursor, doc.as_bytes());
        assert_eq!(
            symbols,
            vec![
                #[allow(deprecated)]
                DocumentSymbol {
                    name: "h1eading".to_string(),
                    detail: Some("* h1eading\n        helo\n".to_string()),
                    kind: SymbolKind::STRUCT,
                    tags: None,
                    deprecated: None,
                    range: range!(1, 8, 3, 0),
                    selection_range: range!(1, 8, 3, 0),
                    children: None,
                },
                #[allow(deprecated)]
                DocumentSymbol {
                    name: "h2eading".to_string(),
                    detail: Some("* h2eading\n        ** h2-1eading\n        ".to_string()),
                    kind: SymbolKind::STRUCT,
                    tags: None,
                    deprecated: None,
                    range: range!(3, 8, 5, 8),
                    selection_range: range!(3, 8, 5, 8),
                    children: Some(vec![DocumentSymbol {
                        name: "h2-1eading".to_string(),
                        detail: Some("** h2-1eading\n        ".to_string()),
                        kind: SymbolKind::STRUCT,
                        tags: None,
                        deprecated: None,
                        range: range!(4, 8, 5, 8),
                        selection_range: range!(4, 8, 5, 8),
                        children: None,
                    }]),
                },
            ]
        );
    }

    // #[test]
    // fn references() {
    //     let current_dir = std::env::current_dir().expect("failed to get current dir");
    //     let workspace = Workspace::from(current_dir.join("./test"));
    //     let location = Location {
    //         uri: Url::from_file_path(current_dir.join("test/folder/foo.norg")).unwrap(),
    //         range: Default::default(),
    //     };
    //     let mut refs = list_references_from_location(&workspace, location);
    //     // sort because file iterator might differ by environment
    //     refs.sort_by_key(|loc| format!("{loc:?}"));
    //     assert_eq!(
    //         refs,
    //         vec![
    //             Location {
    //                 uri: url!("test/folder/bar.norg"),
    //                 range: range!(0, 0, 0, 7),
    //             },
    //             Location {
    //                 uri: url!("test/index.norg"),
    //                 range: range!(7, 2, 7, 16),
    //             },
    //             Location {
    //                 uri: url!("test/index.norg"),
    //                 range: range!(8, 2, 8, 18),
    //             },
    //             Location {
    //                 uri: url!("test/index.norg"),
    //                 range: range!(9, 2, 9, 18),
    //             },
    //         ]
    //     )
    // }
}
