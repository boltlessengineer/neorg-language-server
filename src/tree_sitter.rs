use std::sync::{Arc, Mutex, OnceLock};

use tree_sitter::{Node, Parser};

use crate::document::Document;

#[derive(Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl From<lsp_types::Position> for Position {
    fn from(value: lsp_types::Position) -> Self {
        Position {
            line: value.line as usize,
            col: value.character as usize,
        }
    }
}
impl From<Position> for tree_sitter::Point {
    fn from(value: Position) -> Self {
        tree_sitter::Point {
            row: value.line,
            column: value.col,
        }
    }
}

pub fn ts_to_lsp_range(ts_range: tree_sitter::Range) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position {
            line: ts_range.start_point.row as u32,
            character: ts_range.start_point.column as u32,
        },
        end: lsp_types::Position {
            line: ts_range.end_point.row as u32,
            character: ts_range.end_point.column as u32,
        },
    }
}

#[derive(Debug)]
pub enum LinkDestination {
    Uri(String),
    NorgFile {
        workspace: Option<String>,
        path: String,
        // TODO: scoped location
    },
    // TODO: scope location
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Link {
    range: tree_sitter::Range,
    destination: LinkDestination,
}

impl Document {
    pub fn get_node_from_range(&self, pos: Position) -> Option<Node<'_>> {
        let root = self.tree.root_node();
        root.descendant_for_point_range(pos.into(), pos.into())
    }
    pub fn get_named_node_from_pos(&self, pos: Position) -> Option<Node<'_>> {
        let root = self.tree.root_node();
        root.named_descendant_for_point_range(pos.into(), pos.into())
    }
    pub fn get_link_from_pos(&self, pos: Position) -> Option<Link> {
        let current_node = self.get_named_node_from_pos(pos)?;
        let link_node = match current_node.kind() {
            "uri" | "norg_file" | "workspace" | "norg_file_path" => current_node.parent()?,
            "link" => current_node,
            _ => return None,
        };
        let destination = link_node.child_by_field_name("destination")?;
        return Some(Link {
            range: link_node.range(),
            destination: match destination.kind() {
                "uri" => LinkDestination::Uri(
                    destination
                        .utf8_text(&self.text.to_string().as_bytes())
                        .unwrap()
                        .to_string(),
                ),
                "norg_file" => LinkDestination::NorgFile {
                    workspace: destination.child_by_field_name("workspace").map(|n| {
                        n.utf8_text(&self.text.to_string().as_bytes())
                            .unwrap()
                            .to_string()
                    }),
                    path: destination
                        .child_by_field_name("path")
                        .unwrap()
                        .utf8_text(&self.text.to_string().as_bytes())
                        .unwrap()
                        .to_string(),
                },
                t => todo!("unsupported link type: {t}"),
            },
        });
    }
}

pub static PARSER: OnceLock<Arc<Mutex<Parser>>> = OnceLock::new();
pub fn init_parser() {
    let language = tree_sitter_norg3::language();
    let mut parser = Parser::new();
    parser
        .set_language(language)
        .expect("could not load norg parser");
    let _ = PARSER.set(Arc::new(Mutex::new(parser)));
}

#[cfg(test)]
mod test {
    use super::*;
    use ropey::Rope;

    #[test]
    fn get_node_from_range() {
        let doc_str = String::from(
            r#"
@code lang

@end
"#,
        );
        let language = tree_sitter_norg3::language();
        let mut parser = Parser::new();
        parser
            .set_language(language)
            .expect("could not load norg parser");
        let tree = parser.parse(&doc_str, None).expect("get tree");
        let root = tree.root_node();
        println!("{}", root.to_sexp());
        let doc = Document {
            text: Rope::from_str(&doc_str),
            tree,
        };
        let pos = Position { line: 2, col: 0 };
        let node = doc.get_node_from_range(pos).unwrap();
        println!("{}", node.to_sexp());
    }
}
