use std::path::{Path, PathBuf};

use lsp_types::{Location, Position, Url};
use neorg_dirman::workspace::Workspace;

use crate::{
    norg::{LinkDestination, LinkWorkspace, NorgFile},
    range::Range,
    session::Session,
    syntax::{classify_for_decl, Syntax},
    tree_sitter::{ToLspRange, ToLspRangeWith as _},
    workspace::WorkspaceExt as _,
};

// (copied from rust-analyzer crates/ide/src/lib.rs)
// As a general design guideline, `Analysis` API are intended to be independent
// from the language server protocol. That is, when exposing some functionality
// we should think in terms of "what API makes most sense" and not in terms of
// "what types LSP uses". Although currently LSP is the only consumer of the
// API, the API should in theory be usable as a library, or via a different
// protocol.

pub fn definition(session: &Session, uri: Url, pos: Position) -> Option<Vec<Location>> {
    // 1. from given url & pos, get linkable(reference)
    // 2. if linkable is an anchor reference, return matching anchor definition
    // 3. query location with that target
    //    (this should be done with global session to handle multiple workspaces)
    // 4. return queried location as definition
    let doc = session.get_document(&uri).unwrap();
    let syntax = classify_for_decl(&doc.tree, pos.into())?;
    match syntax {
        Syntax::Link(node) | Syntax::AnchorDefinition(node) => {
            let target_node = node.child_by_field_name("target")?;
            let target =
                LinkDestination::try_from_node(target_node, doc.text.to_string().as_bytes())
                    .ok()?;
            follow_link_target(&session, &uri, &target).map(|loc| vec![loc])
        }
        Syntax::AnchorReference(node) => {
            // go to anchor definition
            let markup_node = node.child_by_field_name("markup")?;
            let markup = markup_node.utf8_text(doc.text.to_string().as_bytes()).unwrap().to_string();
            let def_node = doc.find_anchor_definition(&markup)?;
            let loc = Location::new(uri, def_node.range.to_lsp_range_with(&doc.text));
            log::error!("{loc:?}");
            Some(vec![loc])
        }
        _ => None,
    }
}

pub fn references(session: &Session, req_uri: Url, pos: Position) -> Option<Vec<Location>> {
    // 1. from given url & pos, get linkable(reference) or referenceable
    //    > for linkable(reference), get definition for it using same logic from `definition()`
    // 2. find all linkables pointing to gained definition (referenceable)
    let doc = session.get_document(&req_uri).unwrap();
    let syntax = classify_for_decl(&doc.tree, pos.into())?;
    let origin_loc = match syntax {
        // reference
        Syntax::Link(node) | Syntax::AnchorDefinition(node) => {
            let target_node = node.child_by_field_name("target")?;
            let target =
                LinkDestination::try_from_node(target_node, doc.text.to_string().as_bytes())
                    .ok()?;
            follow_link_target(&session, &req_uri, &target)?
        }
        // referenceable
        Syntax::Section(node) => {
            let range = Range::from(node.range());
            lsp_types::Location::new(req_uri.clone(), range.into())
        }
        // reference of a reference
        Syntax::AnchorReference(..) => {
            todo!("what should I do with anchor reference")
        }
    };
    Some(
        if let Some(workspace) = find_workspace_for_uri(&req_uri) {
            workspace
                .iter_linkables_with(|uri| session.get_document(uri).cloned())
                .filter(|(uri, link)| {
                    follow_link_target(session, &uri, &link.target)
                        .is_some_and(|loc| loc == origin_loc)
                })
                .map(|(uri, link)| Location::new(uri.clone(), link.range.to_lsp_range()))
                .collect()
        } else {
            doc.links
                .iter()
                .filter(|link| {
                    follow_link_target(session, &req_uri, &link.target)
                        .is_some_and(|loc| loc == origin_loc)
                })
                .map(|link| Location::new(req_uri.clone(), link.range.to_lsp_range()))
                .collect()
        }
    )
}

pub fn follow_link_target(
    _session: &Session,
    origin: &Url,
    target: &LinkDestination,
) -> Option<lsp_types::Location> {
    Some(match target {
        LinkDestination::Uri(uri) => lsp_types::Location {
            uri: Url::parse(&uri).ok()?,
            range: Default::default(),
        },
        LinkDestination::Scoped {
            file: Some(NorgFile { root, path }),
            scope: _,
        } => {
            let real_path = if !path.ends_with("norg") {
                path.clone() + ".norg"
            } else {
                path.clone()
            };
            let uri = match root {
                None => {
                    let path = origin.join(&real_path).ok()?;
                    log::error!("{path}");
                    path
                }
                Some(LinkWorkspace::Current) => {
                    let workspace = find_workspace_for_uri(&origin)?;
                    let workspace_url = workspace.get_url().ok()?;
                    workspace_url.join(&real_path).ok()?
                }
                Some(LinkWorkspace::Workspace(_name)) => {
                    todo!("get workspace name from session")
                }
            };
            lsp_types::Location {
                uri,
                range: Default::default(),
            }
        }
        LinkDestination::Scoped {
            file: None,
            scope: _,
        } => {
            unimplemented!("scope is not implemented yet")
        }
    })
}

fn find_workspace_for_uri(uri: &Url) -> Option<Workspace> {
    let path = uri.to_file_path().ok()?;
    fn find_in_parent_dirs(path: &Path, target_file_name: &str) -> Option<PathBuf> {
        if path.file_name().unwrap_or_default() == target_file_name {
            return Some(path.parent().unwrap().to_path_buf());
        }

        let mut curr = Some(path);

        while let Some(path) = curr {
            let candidate = path.join(target_file_name);
            if std::fs::metadata(&candidate).is_ok() {
                return Some(path.to_path_buf());
            }
            curr = path.parent();
        }

        None
    }
    if let Some(path) = find_in_parent_dirs(&path, "root.toml") {
        return Some(Workspace::from(path));
    }
    return None;
}
