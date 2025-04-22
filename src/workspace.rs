use lsp_types::Url;
use neorg_dirman::workspace::Workspace;

use crate::tree_sitter::{LinkDestination, LinkWorkspace, NorgFile};

pub trait WorkspaceExt {
    fn get_url(&self) -> Result<Url, ()>;
    fn resolve_link_location(
        &self,
        origin: &Url,
        link: &LinkDestination,
    ) -> Option<lsp_types::Location>;
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
}
