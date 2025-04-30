use lsp_types::Url;
use neorg_dirman::workspace::Workspace;

use crate::document::{Document, ResolvedLinkable};

pub trait WorkspaceExt {
    fn get_url(&self) -> Result<Url, ()>;
    fn iter_linkables_with(
        &self,
        doc_provider: impl Fn(&Url) -> Option<Document>
    ) -> impl Iterator<Item = (Url, ResolvedLinkable)>;
    fn iter_docs_with(
        &self,
        doc_provider: impl Fn(&Url) -> Option<Document>,
    ) -> impl Iterator<Item = (Url, Document)>;
}

impl WorkspaceExt for Workspace {
    fn get_url(&self) -> Result<Url, ()> {
        Url::from_directory_path(&self.path)
    }
    fn iter_docs_with(
        &self,
        doc_provider: impl Fn(&Url) -> Option<Document>,
    ) -> impl Iterator<Item = (Url, Document)> {
        self.iter_files()
            .flat_map(move |path| {
                let url = Url::from_file_path(path).ok()?;
                let doc = doc_provider(&url).or({
                    let path = url.to_file_path().ok()?;
                    Document::try_from(path.as_path()).ok()
                })?;
                Some((url, doc))
            })
    }
    fn iter_linkables_with(
        &self,
        doc_provider: impl Fn(&Url) -> Option<Document>,
    ) -> impl Iterator<Item = (Url, ResolvedLinkable)> {
        self.iter_docs_with(doc_provider)
            .flat_map(|(url, doc)| {
                doc.links.into_iter().map(move |rl| (url.clone(), rl))
            })
    }
}
