use std::path::Path;

use anyhow::anyhow;
use ropey::RopeSlice;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, TextProvider};

use crate::document::Document;

pub trait PositionTrait {
    fn line(&self) -> usize;
    fn col(&self) -> usize;
    fn as_ts_point(&self) -> tree_sitter::Point {
        tree_sitter::Point {
            row: self.line(),
            column: self.col(),
        }
    }
    fn as_lsp_pos(&self) -> lsp_types::Position {
        lsp_types::Position {
            line: self.line() as u32,
            character: self.col() as u32,
        }
    }
}

impl PositionTrait for lsp_types::Position {
    fn line(&self) -> usize {
        self.line as usize
    }
    fn col(&self) -> usize {
        self.character as usize
    }
}
impl PositionTrait for tree_sitter::Point {
    fn line(&self) -> usize {
        self.row
    }
    fn col(&self) -> usize {
        self.column
    }
}

pub trait ToLspRange {
    fn to_lsp_range(&self) -> lsp_types::Range;
}
impl ToLspRange for tree_sitter::Range {
    fn to_lsp_range(&self) -> lsp_types::Range {
        lsp_types::Range {
            start: self.start_point.as_lsp_pos(),
            end: self.end_point.as_lsp_pos(),
        }
    }
}

pub fn new_norg3_query(source: &str) -> Query {
    Query::new(&tree_sitter_norg::LANGUAGE.into(), source).expect("can't generate query")
}

#[derive(Debug, Clone)]
pub enum LinkDestination {
    Uri(String),
    Scoped {
        file: Option<NorgFile>,
        scope: Vec<LinkScope>,
    },
}

#[derive(Debug, Clone)]
pub struct NorgFile {
    pub root: Option<LinkWorkspace>,
    pub path: String,
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

#[derive(Debug, Clone)]
pub enum LinkWorkspace {
    /// $/
    Current,
    /// $foo/
    Workspace(String),
}

#[derive(Debug, Clone)]
pub enum LinkScope {
    Heading(u16, String),
    WikiHeading(String),
}

#[derive(Debug, Clone)]
pub struct Link {
    pub range: tree_sitter::Range,
    pub dest_range: tree_sitter::Range,
    pub destination: LinkDestination,
    // pub origin: Url,
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

impl LinkDestination {
    pub fn update_uri(&mut self, new_uri: &str) -> anyhow::Result<()> {
        match self {
            Self::Uri(uri) => {
                *uri = new_uri.to_owned();
                Ok(())
            }
            #[allow(unused_variables)]
            Self::Scoped {
                file: Some(NorgFile { root, path }),
                scope,
            } => {
                // TODO:
                // 1. find workspace and relative path of `new_uri` from dirman
                // 2. update `root` and `path` with result
                // Url::parse(&new_uri).unwrap().path().starts_with(new_uri);
                todo!("update norg_file type link destination")
            }
            Self::Scoped {
                file: None,
                scope: _,
            } => Err(anyhow!("Link has no path value")),
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

impl Link {
    fn parse_from_node<'a>(node: Node<'_>, source: &'a [u8]) -> anyhow::Result<Self> {
        let target_node = node
            .child_by_field_name("target")
            .ok_or(anyhow!("`target` field doesn't exist"))?;
        return Ok(Link {
            range: node.range(),
            dest_range: target_node.range(),
            destination: match target_node.kind() {
                "raw_target" => {
                    LinkDestination::Uri(target_node.utf8_text(source).unwrap().to_string())
                }
                "scoped_target" => {
                    let mut iter = ScopedLinkTargetIterator {
                        node: Some(target_node),
                    }
                    .peekable();
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
                            _ => return Err(anyhow!("invalid node for link sco[e")),
                        })
                    }
                    LinkDestination::Scoped { file, scope }
                }
                t => todo!("unsupported link type: {t}"),
            },
        });
    }
}

// copied from helix-editor/helix
// Adapter to convert rope chunks to bytes
pub struct ChunksBytes<'a> {
    chunks: ropey::iter::Chunks<'a>,
}
impl<'a> Iterator for ChunksBytes<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(str::as_bytes)
    }
}

pub struct RopeProvider<'a>(RopeSlice<'a>);
impl<'a> TextProvider<&'a [u8]> for RopeProvider<'a> {
    type I = ChunksBytes<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        let fragment = self.0.byte_slice(node.start_byte()..node.end_byte());
        ChunksBytes {
            chunks: fragment.chunks(),
        }
    }
}

pub fn capture_links(node: Node<'_>, slice: RopeSlice<'_>) -> Vec<Link> {
    let query_str = r#"
    ; query
    [
      (link)
      (unclosed_link)
    ] @link
    "#;
    let query = new_norg3_query(query_str);
    let mut qry_cursor = QueryCursor::new();
    let mut links = vec![];
    let mut matches = qry_cursor.matches(&query, node, RopeProvider(slice));
    while let Some(mat) = matches.next() {
        let mut nodes: Vec<_> = mat
            .captures
            .iter()
            .map(|cap| cap.node)
            .filter_map(|node| Link::parse_from_node(node, slice.to_string().as_bytes()).ok())
            .collect();
        links.append(&mut nodes);
    }
    links
}

impl Document {
    pub fn get_node_from_range<P: PositionTrait>(&self, pos: P) -> Option<Node<'_>> {
        let root = self.tree.root_node();
        root.descendant_for_point_range(pos.as_ts_point(), pos.as_ts_point())
    }
    pub fn get_named_node_from_pos<P: PositionTrait>(&self, pos: P) -> Option<Node<'_>> {
        let root = self.tree.root_node();
        root.named_descendant_for_point_range(pos.as_ts_point(), pos.as_ts_point())
    }
    /// get specific kind of parent node from position
    pub fn get_kind_node_from_pos<P: PositionTrait>(
        &self,
        pos: P,
        kind: &str,
        until: Vec<&str>,
    ) -> Option<Node<'_>> {
        let current_node = self.get_named_node_from_pos(pos)?;
        let mut cursor = current_node.walk();
        loop {
            if cursor.node().kind() == kind {
                return Some(cursor.node());
            }
            if until.contains(&cursor.node().kind()) {
                break;
            }
            // HACK: `!cursor.goto_parent()` doesn't work on node as field content
            if let Some(parent) = cursor.node().parent() {
                cursor = parent.walk();
            } else {
                break;
            }
        }
        None
    }
    pub fn get_link_from_pos<P: PositionTrait>(&self, pos: P) -> Option<Link> {
        log::error!("get_link_from_pos");
        let node = self.get_kind_node_from_pos(pos, "link", vec!["paragraph"])?;
        let link = Link::parse_from_node(node, self.text.to_string().as_bytes());
        match link {
            Ok(link) => Some(link),
            Err(err) => {
                log::error!("{err}");
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tree_sitter::Parser;

    #[test]
    fn get_link_from_pos() {
        let doc = Document::new("{:file:}");
        let link = doc.get_link_from_pos(lsp_types::Position::new(0, 2));
        assert!(!matches!(link, None));
    }

    #[test]
    fn get_node_from_range() {
        let doc_str = String::from(
            r#"
@code lang

@end
"#,
        );
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_norg::LANGUAGE.into())
            .expect("could not load norg parser");
        let tree = parser.parse(&doc_str, None).expect("get tree");
        let root = tree.root_node();
        println!("{}", root.to_sexp());
        let doc = Document::new(&doc_str);
        let pos = lsp_types::Position {
            line: 2,
            character: 0,
        };
        let node = doc.get_node_from_range(pos).unwrap();
        println!("{}", node.to_sexp());
    }

    #[test]
    fn query_links() {
        // cases to match
        // {|
        // {:|
        // {:word|
        // {:word|}
        // {:word|:}
        // {:word|word:}
        // {:word|word} <- bit weird, but would be useful to have

        // TODO: write test
    }
}
