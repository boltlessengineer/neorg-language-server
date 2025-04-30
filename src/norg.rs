use std::path::Path;

use anyhow::{anyhow, Context};
use tree_sitter::Node;

#[derive(PartialEq)]
pub enum Linkable {
    Link {
        target: LinkDestination,
        markup: Option<String>,
        range: tree_sitter::Range,
    },
    Anchor {
        target: Option<LinkDestination>,
        markup: String,
        range: tree_sitter::Range,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkDestination {
    Uri(String),
    Scoped {
        file: Option<NorgFile>,
        scope: Vec<LinkScope>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct NorgFile {
    pub root: Option<LinkWorkspace>,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkWorkspace {
    /// $/
    Current,
    /// $foo/
    Workspace(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkScope {
    Heading(u16, String),
    WikiHeading(String),
}

impl Linkable {
    pub fn range(&self) -> tree_sitter::Range {
        match self {
            Linkable::Link { range, .. } => *range,
            Linkable::Anchor { range, .. } => *range,
        }
    }
    pub fn try_from_node<'src>(node: Node<'_>, source: &'src [u8]) -> anyhow::Result<Self> {
        match node.kind() {
            "link" => {
                let target = node
                    .child_by_field_name("target")
                    .context("can't find 'target' field from node")
                    .and_then(|node| LinkDestination::try_from_node(node, source))?;
                let markup = node
                    .child_by_field_name("markup")
                    .map(|node| node.utf8_text(source).unwrap().to_string());
                Ok(Self::Link {
                    target,
                    markup,
                    range: node.range(),
                })
            }
            "anchor" => {
                let target = node
                    .child_by_field_name("target")
                    .map(|node| LinkDestination::try_from_node(node, source))
                    .transpose()?;
                let markup = node
                    .child_by_field_name("markup")
                    .map(|node| node.utf8_text(source).unwrap().to_string())
                    .context("can't find 'markup' field from node")?;
                Ok(Self::Anchor {
                    target,
                    markup,
                    range: node.range(),
                })
            }
            kind => Err(anyhow!("can't convert node {kind} to linkable")),
        }
    }
}

impl ToString for NorgFile {
    fn to_string(&self) -> String {
        if let Some(root) = &self.root {
            root.to_string() + &self.path
        } else {
            self.path.clone()
        }
    }
}

impl ToString for LinkWorkspace {
    fn to_string(&self) -> String {
        match self {
            Self::Current => "$/".to_owned(),
            Self::Workspace(workspace) => format!("!${workspace}/"),
        }
    }
}

impl ToString for LinkScope {
    fn to_string(&self) -> String {
        match self {
            Self::Heading(level, text) => format!("{} {text}", "*".repeat((*level).into())),
            Self::WikiHeading(text) => format!("? {text}"),
        }
    }
}

impl ToString for LinkDestination {
    fn to_string(&self) -> String {
        match self {
            Self::Uri(uri) => uri.to_owned(),
            Self::Scoped {
                file: Some(file),
                scope,
            } => {
                file.to_string()
                    + &scope
                        .iter()
                        .map(|s| format!(" : {}", s.to_string()))
                        .collect::<String>()
            }
            Self::Scoped { file: None, scope } => scope
                .iter()
                .map(|s| format!(" : {}", s.to_string()))
                .collect(),
        }
    }
}

struct ScopedLinkTargetIterator<'a> {
    node: Option<Node<'a>>,
}

impl<'a> Iterator for ScopedLinkTargetIterator<'a> {
    type Item = Node<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let scope_node = self.node?;
        self.node = scope_node.named_child(1);
        scope_node.named_child(0)
    }
}

impl LinkDestination {
    pub fn try_from_node<'src>(node: Node<'_>, source: &'src [u8]) -> anyhow::Result<Self> {
        match node.kind() {
            "raw_target" => {
                Ok(Self::Uri(node.utf8_text(source).unwrap().to_string()))
            }
            "scoped_target" => {
                let mut iter = ScopedLinkTargetIterator {
                    node: Some(node),
                }.peekable();
                let file =
                    if iter.peek().ok_or(anyhow!("scope is empty"))?.kind() == "raw_target" {
                        let first = iter.next().unwrap();
                        let raw_path = first.utf8_text(source).unwrap();
                        let (workspace, path) = if raw_path.starts_with("$/") {
                            (Some(LinkWorkspace::Current), &raw_path[2..])
                        } else if raw_path.starts_with("$") {
                            let name = Path::new(&raw_path[1..])
                                .iter()
                                .next()
                                .unwrap()
                                .to_string_lossy()
                                .into_owned();
                            let path = raw_path.trim_start_matches(&format!("${name}"));
                            (Some(LinkWorkspace::Workspace(name)), path)
                        } else {
                            (None, raw_path)
                        };
                        Some(NorgFile {
                            root: workspace,
                            path: path.to_string(),
                        })
                    } else {
                        None
                    };
                let mut scope = vec![];
                while let Some(node) = iter.next() {
                    scope.push(match node.kind() {
                        "heading_target" => {
                            let prefix_node = node.child(0).unwrap();
                            let level =
                                prefix_node.range().end_byte - prefix_node.range().start_byte;
                            let text_node = node.child_by_field_name("text").unwrap();
                            let text = text_node.utf8_text(source).unwrap().to_string();
                            LinkScope::Heading(level as u16, text)
                        }
                        "wiki_target" => {
                            let text_node = node.child_by_field_name("text").unwrap();
                            let text = text_node.utf8_text(source).unwrap().to_string();
                            LinkScope::WikiHeading(text)
                        }
                        n => return Err(anyhow!("invalid node {n} for link scope")),
                    })
                }
                Ok(Self::Scoped { file, scope })
            }
            t => Err(anyhow!("unsupported link type: {t}")),
        }
    }

    // pub fn update_uri(&mut self, new_uri: &str) -> anyhow::Result<()> {
    //     match self {
    //         Self::Uri(uri) => {
    //             *uri = new_uri.to_owned();
    //             Ok(())
    //         }
    //         #[allow(unused_variables)]
    //         Self::Scoped {
    //             file: Some(NorgFile { root, path }),
    //             scope,
    //         } => {
    //             // TODO:
    //             // 1. find workspace and relative path of `new_uri` from dirman
    //             // 2. update `root` and `path` with result
    //             // Url::parse(&new_uri).unwrap().path().starts_with(new_uri);
    //             todo!("update norg_file type link destination")
    //         }
    //         Self::Scoped {
    //             file: None,
    //             scope: _,
    //         } => Err(anyhow!("Link has no path value")),
    //     }
    // }
}
