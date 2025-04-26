use ropey::{Rope, RopeSlice};
use tree_sitter::{Node, Query, TextProvider};

// TODO: replace these traits with actual internal structs
pub trait RangeTrait {
    type Pos: PositionTrait;
    fn start(&self) -> Self::Pos;
    fn end(&self) -> Self::Pos;
}

impl RangeTrait for tree_sitter::Range {
    type Pos = tree_sitter::Point;

    fn start(&self) -> Self::Pos {
        self.start_point
    }

    fn end(&self) -> Self::Pos {
        self.end_point
    }
}

impl RangeTrait for lsp_types::Range {
    type Pos = lsp_types::Position;

    fn start(&self) -> Self::Pos {
        self.start
    }

    fn end(&self) -> Self::Pos {
        self.end
    }
}

pub trait PositionTrait: Copy {
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

// I want all PositionTraits to automatically implement RangeTrait
impl<P> RangeTrait for P
where
    P: PositionTrait
{
    type Pos = P;

    fn start(&self) -> Self::Pos {
        *self
    }

    fn end(&self) -> Self::Pos {
        *self
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
impl<'a> From<&'a Rope> for RopeProvider<'a> {
    fn from(rope: &'a Rope) -> Self {
        Self(rope.slice(..))
    }
}
impl<'a> TextProvider<&'a [u8]> for RopeProvider<'a> {
    type I = ChunksBytes<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        let fragment = self.0.byte_slice(node.start_byte()..node.end_byte());
        ChunksBytes {
            chunks: fragment.chunks(),
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use tree_sitter::Parser;
//
//     #[test]
//     fn get_link_from_pos() {
//         let doc = Document::new("{:file:}");
//         let link = doc.get_link_from_pos(lsp_types::Position::new(0, 2));
//         assert!(!matches!(link, None));
//     }
//
//     #[test]
//     fn get_node_from_range() {
//         let doc_str = String::from(
//             r#"
// @code lang
//
// @end
// "#,
//         );
//         let mut parser = Parser::new();
//         parser
//             .set_language(&tree_sitter_norg::LANGUAGE.into())
//             .expect("could not load norg parser");
//         let tree = parser.parse(&doc_str, None).expect("get tree");
//         let root = tree.root_node();
//         println!("{}", root.to_sexp());
//         let doc = Document::new(&doc_str);
//         let pos = lsp_types::Position {
//             line: 2,
//             character: 0,
//         };
//         let node = doc.get_named_node_from_range(pos).unwrap();
//         assert_eq!(node.kind(), "ranged_tag");
//     }
//
//     #[test]
//     fn query_links() {
//         // cases to match
//         // {|
//         // {:|
//         // {:word|
//         // {:word|}
//         // {:word|:}
//         // {:word|word:}
//         // {:word|word} <- bit weird, but would be useful to have
//
//         // TODO: write test
//     }
// }
