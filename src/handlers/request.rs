use std::collections::HashMap;

use log::{debug, error};
use lsp_server::Response;
use lsp_types::{
    CompletionItem, CompletionList, CompletionParams, DocumentSymbol, DocumentSymbolParams,
    Documentation, GotoDefinitionParams, GotoDefinitionResponse, InsertTextFormat, Location,
    ReferenceParams, RenameFilesParams, Url,
};

use crate::{
    config::Config,
    document::{Document, DOC_STORE},
    norg::NORG_BLOCKS,
    tree_sitter::{Link, ToLspRange},
    workspace::WS_MANAGER,
};

pub fn handle_completion(req: lsp_server::Request) -> Response {
    let params: CompletionParams = serde_json::from_value(req.params).unwrap();
    let uri = params.text_document_position.text_document.uri;
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
                // also should return the symbol type
                // if field `title` is empty (slide/indent segments,) create
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
        }
    }
    if cursor.goto_next_sibling() {
        symbols.append(&mut tree_to_symbols(cursor, text));
    } else {
        cursor.goto_parent();
    }
    return symbols;
}

pub fn handle_document_symbol(req: lsp_server::Request) -> Response {
    error!("document symbol");
    let params: DocumentSymbolParams = serde_json::from_value(req.params).unwrap();
    let uri = params.text_document.uri;
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
    let doc = doc_store.get(&req_uri).unwrap();
    if let Some(link) = doc.get_link_from_pos(req_pos) {
        debug!("{link:?}");
        match link.get_location(&req_uri) {
            Ok(loc) => {
                let definitions = GotoDefinitionResponse::Scalar(loc);
                Response::new_ok(req.id, serde_json::to_value(definitions).unwrap())
            }
            Err(e) => Response::new_err(
                req.id,
                lsp_server::ErrorCode::RequestFailed as i32,
                e.to_string(),
            ),
        }
    } else {
        Response::new_err(
            req.id,
            lsp_server::ErrorCode::RequestFailed as i32,
            "can't find link in request position".to_string(),
        )
    }
}

pub fn handle_references(_config: &Config, req: lsp_server::Request) -> Response {
    let params: ReferenceParams = serde_json::from_value(req.params).unwrap();
    let req_uri = params.text_document_position.text_document.uri;
    let req_pos = params.text_document_position.position;
    let link_from_pos = {
        // access DOC_STORE from scope to prevent deadlock
        let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
        let doc = doc_store.get(&req_uri).unwrap();
        doc.get_link_from_pos(req_pos)
    };
    if let Some(link) = link_from_pos {
        error!("{link:?}");
        match link.get_location(&req_uri) {
            Ok(req_link_loc) => {
                let references = list_references_from_location(req_link_loc);
                Response::new_ok(req.id, serde_json::to_value(references).unwrap())
            }
            Err(e) => Response::new_err(
                req.id,
                lsp_server::ErrorCode::RequestFailed as i32,
                e.to_string(),
            ),
        }
    } else {
        Response::new_err(
            req.id,
            lsp_server::ErrorCode::InvalidRequest as i32,
            "can't find link in request position".to_string(),
        )
    }
}

fn list_references_from_location(loc: Location) -> Vec<Location> {
    let dirman = WS_MANAGER.get().unwrap().lock().unwrap();
    dirman
        .get_current_workspace()
        .files()
        .into_iter()
        .filter_map(|path| {
            let url = Url::from_file_path(&path).ok()?;
            let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
            // TODO: push document created from path to DOC_STORE
            doc_store
                .get(&url)
                .map(|d| d.clone())
                .or(Document::from_path(&path).ok())
                .map(|d| (url, d))
        })
        .flat_map(|(url, d)| d.links.into_iter().map(move |l| (url.clone(), l)))
        .filter(|(origin, link)| match link.get_location(origin) {
            Ok(link_loc) => link_loc == loc,
            Err(_) => false,
        })
        .map(|(path, l)| lsp_types::Location {
            uri: path,
            range: l.range.as_lsp_range(),
        })
        .collect()
}

fn iter_links() -> impl Iterator<Item = (Url, Link)> {
    let dirman = WS_MANAGER.get().unwrap().lock().unwrap();
    dirman
        .get_current_workspace()
        .files()
        .into_iter()
        .filter_map(|path| {
            let url = Url::from_file_path(&path).ok()?;
            let doc_store = DOC_STORE.get().unwrap().lock().unwrap();
            // TODO: push document created from path to DOC_STORE
            doc_store
                .get(&url)
                .map(|d| d.clone())
                .or(Document::from_path(&path).ok())
                .map(|d| (url, d))
        })
        .flat_map(|(url, d)| d.links.into_iter().map(move |l| (url.clone(), l)))
}

pub fn handle_will_rename_files(req: lsp_server::Request) -> Response {
    error!("willrename");
    let params: RenameFilesParams = serde_json::from_value(req.params).unwrap();
    let mut changes: HashMap<Url, Vec<lsp_types::TextEdit>> = HashMap::new();
    params.files.iter().for_each(|file_rename| {
        iter_links()
            .filter(|(url, link)| match link.get_location(url) {
                Ok(loc) => loc.uri.to_string() == file_rename.old_uri,
                Err(_) => false,
            })
            .for_each(|(link_origin, link)| {
                let mut new_dest = link.destination;
                new_dest.update_uri(&file_rename.new_uri).unwrap();
                let textedit =
                    lsp_types::TextEdit::new(link.dest_range.as_lsp_range(), new_dest.to_string());
                error!("{textedit:?}");
                changes
                    .entry(link_origin)
                    .and_modify(|v| v.push(textedit.clone()))
                    .or_insert(vec![textedit]);
            });
    });
    let workspace_edit = lsp_types::WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    };
    Response::new_ok(req.id, serde_json::to_value(workspace_edit).unwrap())
}

#[cfg(test)]
mod test {
    use lsp_types::{Position, Range, SymbolKind};
    use tree_sitter::Parser;

    use crate::{document::init_doc_store, workspace::init_worksapce};

    use super::*;

    macro_rules! url {
        ($path:expr) => {
            Url::from_file_path(
                std::env::current_dir()
                    .expect("failed to get current dir")
                    .join($path),
            )
            .unwrap()
        };
    }

    macro_rules! range {
        ($sl:expr, $sc:expr, $el:expr, $ec:expr) => {
            Range::new(Position::new($sl, $sc), Position::new($el, $ec))
        };
    }

    #[allow(deprecated)]
    #[test]
    fn document_symbols() {
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
            symbols,
            vec![
                DocumentSymbol {
                    name: "ranged tag".to_string(),
                    detail: Some(
                        "|example\n        * h1eading\n        helo\n        |end\n".to_string()
                    ),
                    kind: SymbolKind::STRUCT,
                    tags: None,
                    deprecated: None,
                    range: range!(0, 0, 4, 0),
                    selection_range: range!(0, 0, 4, 0),
                    children: Some(vec![DocumentSymbol {
                        name: "heading".to_string(),
                        detail: Some("* h1eading\n        helo\n".to_string()),
                        kind: SymbolKind::STRUCT,
                        tags: None,
                        deprecated: None,
                        range: range!(1, 8, 3, 0),
                        selection_range: range!(1, 8, 3, 0),
                        children: None,
                    }]),
                },
                DocumentSymbol {
                    name: "heading".to_string(),
                    detail: Some("* h2eading\n".to_string()),
                    kind: SymbolKind::STRUCT,
                    tags: None,
                    deprecated: None,
                    range: range!(4, 8, 5, 0),
                    selection_range: range!(4, 8, 5, 0),
                    children: None,
                },
            ]
        );
    }

    #[test]
    fn references() {
        // structured_logger::Builder::with_level("debug").init();
        let current_dir = std::env::current_dir().expect("failed to get current dir");
        init_doc_store();
        init_worksapce(current_dir.join("./test"));
        let location = Location {
            uri: Url::from_file_path(current_dir.join("test/folder/foo.norg")).unwrap(),
            range: Default::default(),
        };
        let mut refs = list_references_from_location(location);
        // sort because file iterator might differ by environment
        refs.sort_by_key(|loc| format!("{loc:?}"));
        assert_eq!(
            refs,
            vec![
                Location {
                    uri: url!("test/folder/bar.norg"),
                    range: range!(0, 0, 0, 7),
                },
                // FIXME: first link should be invalid
                // but currently no way to capture nested node with tree-sitter queries
                // see: https://github.com/tree-sitter/tree-sitter/issues/880
                Location {
                    uri: url!("test/index.norg"),
                    range: range!(1, 0, 1, 14),
                },
                //
                Location {
                    uri: url!("test/index.norg"),
                    range: range!(10, 2, 10, 16),
                },
                Location {
                    uri: url!("test/index.norg"),
                    range: range!(11, 2, 11, 18),
                },
            ]
        )
    }
}
