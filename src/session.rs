use std::collections::HashMap;

use lsp_types::Url;
use neorg_dirman::workspace::Workspace;

use crate::document::Document;

// global server state. meant to replace state::State
#[derive(Default)]
pub struct Session {
    index: Index,
    pub workspace: Option<Workspace>,
    // workspaces: BTreeMap<Url, NorgWorkspace>,
}

#[derive(Default)]
pub struct Index {
    documents: HashMap<Url, Document>,
}

impl Session {
    pub fn with_workspace(workspace: Option<Workspace>) -> Self {
        Self { workspace, ..Default::default() }
    }
    pub fn insert_document(&mut self, url: Url, document: Document) -> anyhow::Result<()> {
        self.index.documents.insert(url, document);
        Ok(())
    }
    pub fn get_document(&self, url: &Url) -> Option<&Document> {
        self.index.documents.get(url)
    }
    pub fn update_document(&mut self, url: &Url, changes: Vec<lsp_types::TextDocumentContentChangeEvent>) -> anyhow::Result<()> {
        let Some(doc) = self.index.documents.get_mut(url) else {
            anyhow::bail!("document {url} doesn't exist")
        };
        for change in changes.iter() {
            if let Some(range) = change.range {
                doc.change_range(range, &change.text);
            }
        }
        doc.update();
        Ok(())
    }
    pub fn remove_document(&mut self, url: &Url) -> anyhow::Result<()> {
        self.index.documents.remove(url);
        Ok(())
    }
}
