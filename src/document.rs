use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use log::error;
use ropey::Rope;
use tree_sitter::{InputEdit, Tree};

use crate::tree_sitter::PARSER;

#[derive(Debug)]
pub struct Document {
    pub text: Rope,
    pub tree: Tree,
    // TODO: linkable symbols (e.g. headings) as Vec<Symbol>
    // cached on didChange event
    // headings inside standard ranged tags are ignored
}

impl Document {
    pub fn new(text: String) -> Self {
        // parse and save tree
        let rope = Rope::from_str(&text);
        let mut parser = PARSER.get().unwrap().lock().unwrap();
        let tree = parser.parse(&text, None).unwrap();
        return Document { text: rope, tree };
    }
    fn edit_from_range(&mut self, range: lsp_types::Range, insert: &str) -> InputEdit {
        let start_byte = self.text.line_to_byte(range.start.line as usize) + range.start.character as usize;
        let end_byte = self.text.line_to_byte(range.end.line as usize) + range.end.character as usize;
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
    pub fn update_tree(&mut self) {
        self.tree = PARSER
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .parse(self.text.to_string(), Some(&self.tree))
            .unwrap();
    }
    #[allow(dead_code)]
    pub fn get_links(&self) -> Vec<tree_sitter::Node> {
        return vec![];
    }
    #[allow(dead_code)]
    pub fn get_targets(&self) -> Vec<tree_sitter::Node> {
        return vec![];
    }
}

pub static DOC_STORE: OnceLock<Arc<Mutex<HashMap<String, Document>>>> = OnceLock::new();

pub fn init_doc_store() {
    DOC_STORE.set(Arc::new(Mutex::new(HashMap::new()))).unwrap();
}
