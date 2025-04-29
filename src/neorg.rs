use lsp_types::{Location, Position, Url};

use crate::{session::Session, tree_sitter::ToLspRange, workspace::WorkspaceExt as _};

pub fn definition(session: &Session, uri: Url, pos: Position) -> Option<Vec<Location>> {
    let Some(ref workspace) = session.workspace else {
        todo!();
    };
    let doc = session.get_document(&uri).unwrap();
    let linkable = doc.find_linkable_from_pos(pos)?;
    let Ok(link) = doc.local_resolve_linkable(linkable) else {
        return None;
    };
    workspace.resolve_link_location(&uri, &link.target).map(|loc| vec![loc])
}

pub fn references(session: &Session, req_uri: Url, pos: Position) -> Option<Vec<Location>> {
    let Some(ref workspace) = session.workspace else {
        todo!();
    };
    // 1. find reference (linkables) or referencable (headings, block with ids in future) in
    //    current location
    let doc = session.get_document(&req_uri).unwrap();
    if let Some(linkable) = doc.find_linkable_from_pos(pos) {
        let Ok(resolved_linkable) = doc.local_resolve_linkable(linkable) else {
            return None;
        };
        let req_loc = workspace.resolve_link_location(&req_uri, &resolved_linkable.target);
        // 2-a. for all documents in workspace, find all references that matches the given reference
        Some(
            workspace
                .iter_linkables_with(|uri| {
                    session.get_document(uri).cloned()
                })
                .filter(|(uri, link)| {
                    workspace.resolve_link_location(&uri, &link.target)
                        == req_loc
                })
                .map(|(uri, link)| Location::new(uri.clone(), link.range.to_lsp_range()))
                .collect(),
        )
    } else if let Some(_referencable) = doc.find_referenceable_from_pos(pos) {
        // 2-b. for all documents in workspace, find all references that matches the given referenceable
        // ~ 1. for all references
        todo!()
    } else {
        None
    }
}
