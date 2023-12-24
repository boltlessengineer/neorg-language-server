use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

use log::error;
use lsp_types::Url;
use ropey::Rope;
use tree_sitter::{InputEdit, Parser, Tree};

use crate::tree_sitter::{capture_links, Link};

#[derive(Debug, Clone)]
pub struct Document {
    pub text: Rope,
    pub tree: Tree,
    // TODO: linkable symbols (e.g. headings) as Vec<Symbol>
    // cached on didChange event
    // headings inside standard ranged tags are ignored
    pub links: Vec<Link>,
}

impl Document {
    pub fn new(text: &str) -> Self {
        // parse and save tree
        let rope = Rope::from_str(&text);
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_norg3::language())
            .expect("could not load norg parser");
        let tree = parser.parse(&text, None).unwrap();
        let links = capture_links(tree.root_node(), rope.slice(..));
        return Self {
            text: rope,
            tree,
            links,
        };
    }
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        Ok(Document::new(&std::fs::read_to_string(path)?))
    }
    fn edit_from_range(&mut self, range: lsp_types::Range, insert: &str) -> InputEdit {
        let start_byte =
            self.text.line_to_byte(range.start.line as usize) + range.start.character as usize;
        let end_byte =
            self.text.line_to_byte(range.end.line as usize) + range.end.character as usize;
        let new_end_byte = start_byte + insert.len();
        self.text.try_remove(start_byte..end_byte).unwrap();
        self.text.try_insert(start_byte, insert).unwrap();
        // PERF: calculate end position from inserted text, not whole document
        let new_end_row = self.text.try_byte_to_line(new_end_byte).unwrap();
        let new_end_col = self.text.try_byte_to_char(new_end_byte).unwrap();
        let edit = InputEdit {
            start_byte,
            old_end_byte: end_byte,
            new_end_byte,
            start_position: tree_sitter::Point {
                row: range.start.line as usize,
                column: range.start.character as usize,
            },
            old_end_position: tree_sitter::Point {
                row: range.end.line as usize,
                column: range.end.character as usize,
            },
            new_end_position: tree_sitter::Point {
                row: new_end_row,
                column: new_end_col,
            },
        };
        return edit;
    }

    pub fn change_range(&mut self, range: lsp_types::Range, text: &str) {
        let start_idx =
            self.text.line_to_byte(range.start.line as usize) + range.start.character as usize;
        let end_idx =
            self.text.line_to_byte(range.end.line as usize) + range.end.character as usize;
        error!("change range {:?}..{:?}", start_idx, end_idx);
        let edit = self.edit_from_range(range, text);
        // update text
        // edit tree
        self.tree.edit(&edit);
    }
    /// apply text update to document.
    /// this will re-parse the Tree and capture all links
    pub fn update(&mut self) {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_norg3::language())
            .expect("could not load norg parser");
        self.tree = parser
            .parse(self.text.to_string(), Some(&self.tree))
            .unwrap();
        self.links = capture_links(self.tree.root_node(), self.text.slice(..));
    }
}

pub static DOC_STORE: OnceLock<Arc<Mutex<HashMap<Url, Document>>>> = OnceLock::new();

pub fn init_doc_store() {
    DOC_STORE.set(Arc::new(Mutex::new(HashMap::new()))).unwrap();
}
