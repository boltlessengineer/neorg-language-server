use anyhow::anyhow;
use lsp_types::Url;
// use lsp_types::Url;
use ropey::RopeSlice;
use tree_sitter::{Node, Query, QueryCursor, TextProvider};

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

// TODO: do we really need Range as Trait??
pub trait _Range {
    fn as_ts_range(&self) -> tree_sitter::Range;
    fn as_lsp_range(&self) -> lsp_types::Range;
}
impl _Range for tree_sitter::Range {
    fn as_ts_range(&self) -> tree_sitter::Range {
        self.clone()
    }
    fn as_lsp_range(&self) -> lsp_types::Range {
        lsp_types::Range {
            start: self.start_point.as_lsp_pos(),
            end: self.end_point.as_lsp_pos(),
        }
    }
}

pub fn new_norg3_query(source: &str) -> Query {
    Query::new(tree_sitter_norg3::language(), source).expect("can't generate query")
}

#[derive(Debug)]
pub enum LinkDestination {
    Uri(String),
    NorgFile {
        // TODO: replace to Option<LinkRoot>
        root: LinkRoot,
        path: String,
        // TODO: scoped location
    },
    // TODO: scope location
}

#[derive(Debug)]
pub enum LinkRoot {
    Current,
    Workspace(String),
    Root,
    None,
}

#[derive(Debug)]
pub struct Link {
    #[allow(dead_code)]
    pub range: tree_sitter::Range,
    pub destination: LinkDestination,
    // pub uri: Url,
}

impl Link {
    fn parse_from_node<'a>(node: Node<'_>, text_provider: &'a [u8]) -> Option<Self> {
        let destination = node.child_by_field_name("destination")?;
        return Some(Link {
            range: node.range(),
            destination: match destination.kind() {
                "uri" => {
                    LinkDestination::Uri(destination.utf8_text(text_provider).unwrap().to_string())
                }
                "norg_file" => LinkDestination::NorgFile {
                    root: destination
                        .child_by_field_name("root")
                        .map_or(LinkRoot::None, |node| match node.kind() {
                            "file_root" => LinkRoot::Root,
                            "current_workspace" => LinkRoot::Current,
                            "workspace" => LinkRoot::Workspace(
                                node.utf8_text(text_provider).unwrap().to_string(),
                            ),
                            k => unreachable!("invalid root kind: {k}"),
                        }),
                    path: destination
                        .child_by_field_name("path")
                        .unwrap()
                        .utf8_text(text_provider)
                        .unwrap()
                        .to_string(),
                },
                t => todo!("unsupported link type: {t}"),
            },
        });
    }
    // TODO: change to `to_location` which returns lsp_types::Location
    // containing destination Uri and range
    // Link::get_location(&self, origin: &Url) -> Result<Location>
    // list_reference_from_location(loc: Location, root: P) -> Vec<Location>
    pub fn get_location(&self, origin: &Url) -> anyhow::Result<lsp_types::Location> {
        Ok(match &self.destination {
            LinkDestination::Uri(uri) => lsp_types::Location {
                uri: Url::parse(&uri)?,
                range: Default::default(),
            },
            LinkDestination::NorgFile { root, path } => {
                let path = if path.ends_with(".norg") {
                    path.to_owned()
                } else {
                    path.to_owned() + ".norg"
                };
                let uri = match root {
                    LinkRoot::None => origin.join(&path)?,
                    LinkRoot::Root => Url::parse(&format!("file:///{}", &path))?,
                    LinkRoot::Workspace(_) | LinkRoot::Current => {
                        return Err(anyhow!("workspace links are not implemented yet"))
                    }
                };
                lsp_types::Location {
                    uri,
                    range: Default::default(),
                }
            }
        })
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

pub struct RopeProvider<'a>(pub RopeSlice<'a>);
impl<'a> TextProvider<'a> for RopeProvider<'a> {
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
    (link
        destination: _ @destination
    ) @uri_link
    "#;
    let query = new_norg3_query(query_str);
    let mut qry_cursor = QueryCursor::new();
    let matches = qry_cursor.matches(&query, node, RopeProvider(slice));
    return matches
        .into_iter()
        .flat_map(|m| m.captures.iter().map(|c| c.node))
        .filter_map(|n| Link::parse_from_node(n, slice.to_string().as_bytes()))
        .collect();
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
        // TODO: Vec instead of Option
        until: Option<&str>,
    ) -> Option<Node<'_>> {
        let current_node = self.get_named_node_from_pos(pos)?;
        let mut cursor = current_node.walk();
        loop {
            if cursor.node().kind() == kind {
                return Some(cursor.node());
            }
            // HACK: `!cursor.goto_parent()` doesn't work on node as field content
            if until == Some(cursor.node().kind()) {
                break;
            }
            if let Some(parent) = cursor.node().parent() {
                cursor = parent.walk();
            } else {
                break;
            }
        }
        None
    }
    pub fn get_link_from_pos<P: PositionTrait>(&self, pos: P) -> Option<Link> {
        let node = self.get_kind_node_from_pos(pos, "link", Some("paragraph"))?;
        return Link::parse_from_node(node, self.text.to_string().as_bytes());
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
            .set_language(tree_sitter_norg3::language())
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
        let doc_str = r#"
        word{

        word{:

        word {}

        {:word:}
        "#;
        let doc = Document::new(&doc_str);
        let root = doc.tree.root_node();
        println!("{}", root.to_sexp());
        let query_str = r#"
            ; query
            (link
                destination: (norg_file)
            ) @_link

            (link
                destination: (uri)
            ) @_link

            (
                (ERROR "{")
                (punc "}")?
            ) @_link
        "#;
        let query = new_norg3_query(query_str);
        let mut qry_cursor = QueryCursor::new();
        let list: Vec<Node> = qry_cursor
            .matches(&query, root, doc.text.to_string().as_bytes())
            .into_iter()
            .flat_map(|m| {
                println!("{m:?}");
                m.captures.iter().map(|c| c.node)
            })
            .collect();
        list.iter().for_each(|n| {
            println!("node:{}", n.to_sexp());
        })
    }
}
