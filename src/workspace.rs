use lsp_types::Url;
use neorg_dirman::workspace::Workspace;

use crate::{document::Document, norg::{LinkDestination, LinkWorkspace, Linkable, NorgFile}};

pub trait WorkspaceExt {
    fn get_url(&self) -> Result<Url, ()>;
    fn resolve_link_location(
        &self,
        origin: &Url,
        link: &LinkDestination,
    ) -> Option<lsp_types::Location>;
    fn iter_linkables_with(
        &self,
        doc_provider: impl Fn(&Url) -> Option<Document>
    ) -> impl Iterator<Item = (Url, Linkable)>;
}

impl WorkspaceExt for Workspace {
    fn get_url(&self) -> Result<Url, ()> {
        Url::from_directory_path(&self.path)
    }
    fn resolve_link_location(
        &self,
        origin: &Url,
        link: &LinkDestination,
    ) -> Option<lsp_types::Location> {
        Some(match link {
            LinkDestination::Uri(uri) => lsp_types::Location {
                uri: Url::parse(&uri).ok()?,
                range: Default::default(),
            },
            LinkDestination::Scoped {
                file: Some(NorgFile { root, path }),
                scope: _,
            } => {
                let path = if path.ends_with(".norg") {
                    path.to_owned()
                } else {
                    path.to_owned() + ".norg"
                };
                let uri = match root {
                    None => {
                        let path = origin.join(&path).ok()?;
                        log::error!("{path}");
                        path
                    }
                    Some(LinkWorkspace::Workspace(_name)) => {
                        unimplemented!("external workspace is not implemented yet")
                    }
                    Some(LinkWorkspace::Current) => {
                        let url = self.get_url().ok()?;
                        log::error!("{url}");
                        log::error!("{path}");
                        url.join(&path).ok()?
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
    // TODO: fix: this should iterate through resolved linkables
    fn iter_linkables_with(
        &self,
        doc_provider: impl Fn(&Url) -> Option<Document>,
    ) -> impl Iterator<Item = (Url, Linkable)> {
        self.iter_files()
            .flat_map(move |path| {
                let url = Url::from_file_path(path).ok()?;
                let doc = doc_provider(&url).or({
                    let path = url.to_file_path().ok()?;
                    Document::try_from(path.as_path()).ok()
                })?;
                Some((url, doc))
            })
            .flat_map(|(url, doc)| {
                doc.iter_linkables()
                    .map(move |linkable| (url.clone(), linkable))
            })
    }
}
