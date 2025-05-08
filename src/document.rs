use std::path::Path;

use norg_rs::parser::Markup;
use ropey::Rope;
use tree_sitter::{InputEdit, QueryCursor, StreamingIterator, Tree};

use crate::{
    norg::{LinkDestination, Linkable},
    tree_sitter::{new_norg3_query, parse_norg, RopeExt, RopeProvider, ToLspRange as _},
};

// TODO: Revisit to this type. I might not need to resolve linkables at all
#[derive(Debug, Clone)]
pub struct ResolvedLinkable {
    pub target: LinkDestination,
    pub range: tree_sitter::Range,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub text: Rope,
    pub tree: Tree,
    // TODO: linkable symbols (e.g. headings) as Vec<Symbol> cached on document change
    pub links: Vec<ResolvedLinkable>,
}

impl Document {
    pub fn new(text: &str) -> Self {
        // parse and save tree
        let rope = Rope::from_str(&text);
        let tree = parse_norg(&text, None).unwrap();
        let links = vec![];
        let mut doc = Self {
            text: rope,
            tree,
            links,
        };
        doc.links = doc.resolved_linkables();
        doc
    }
    fn edit_from_range(&mut self, range: lsp_types::Range, insert: &str) -> InputEdit {
        let start_byte =
            self.text.line_to_byte(range.start.line as usize) + range.start.character as usize;
        let end_byte =
            self.text.line_to_byte(range.end.line as usize) + range.end.character as usize;
        let new_end_byte = start_byte + insert.len();
        self.text.try_remove(start_byte..end_byte).unwrap();
        self.text.try_insert(start_byte, insert).unwrap();
        let (new_end_row, new_end_col) = self.text.try_byte_to_pos(new_end_byte).unwrap();
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
        let edit = self.edit_from_range(range, text);
        // update text
        // edit tree
        self.tree.edit(&edit);
    }
    /// apply text update to document.
    /// this will re-parse the Tree and capture all links
    pub fn update(&mut self) {
        self.tree = parse_norg(self.text.to_string(), Some(&self.tree)).unwrap();
        self.links = self.resolved_linkables();
    }

    pub fn iter_linkables(&self) -> impl Iterator<Item = Linkable> {
        let query_str = r#"
            ; query
            [
              (link)
              (anchor)
            ] @linkable
        "#;
        let query = new_norg3_query(query_str);
        let mut qry_cursor = QueryCursor::new();
        let mut matches = qry_cursor.matches(
            &query,
            self.tree.root_node(),
            RopeProvider::from(&self.text),
        );
        // TODO: avoid collecting
        let mut links = vec![];
        while let Some(mat) = matches.next() {
            links.extend(mat.captures.iter().map(|cap| cap.node).filter_map(|node| {
                Linkable::try_from_node(node, self.text.to_string().as_bytes()).ok()
            }));
        }
        links.into_iter()
    }

    pub fn resolved_linkables(&self) -> Vec<ResolvedLinkable> {
        let linkables: Vec<_> = self.iter_linkables().collect();
        let mut resolved = vec![];
        for linkable in linkables.iter() {
            resolved.push(match linkable {
                Linkable::Link { target, range, .. } => {
                    ResolvedLinkable {
                        target: target.clone(),
                        range: *range,
                    }
                },
                Linkable::Anchor { target: Some(target), range, .. } => {
                    ResolvedLinkable {
                        target: target.clone(),
                        range: *range,
                    }
                },
                Linkable::Anchor { target: None, range, .. } => {
                    let target = linkables.iter().find_map(|linkable| {
                        let Linkable::Anchor { target: Some(target), .. } = linkable else {
                            return None;
                        };
                        Some(target)
                    });
                    let Some(target) = target else {
                        continue;
                    };
                    let target = target.clone();
                    ResolvedLinkable {
                        target,
                        range: *range,
                    }
                },
            })
        }
        resolved
    }

    pub fn get_symbol_tree(&self) -> Vec<lsp_types::DocumentSymbol> {
        let mut cursor = self.tree.walk();
        let bytes: Vec<_> = self.text.bytes().collect();
        tree_to_symbols(&mut cursor, &bytes)
    }

    pub fn find_anchor_definition(&self, markup: &Markup) -> Option<norg_rs::parser::AnchorDefinitionNode> {
        let ast = norg_rs::parser::parse_tstree(&self.tree, self.text.to_string().as_bytes());
        let node = ast.anchors.get(markup)?;
        Some(node.clone())
    }
}

impl TryFrom<&Path> for Document {
    type Error = std::io::Error;
    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Ok(Document::new(&std::fs::read_to_string(path)?))
    }
}

fn tree_to_symbols(cursor: &mut ::tree_sitter::TreeCursor, text: &[u8]) -> Vec<lsp_types::DocumentSymbol> {
    let node = cursor.node();
    let mut symbols: Vec<lsp_types::DocumentSymbol> = vec![];
    if node.is_named() {
        match node.kind() {
            "document" => {
                cursor.goto_first_child();
                return tree_to_symbols(cursor, text);
            }
            "section" => {
                // TODO: return range that can used for `name` and `selection_range`
                // also should return the symbol type
                let title_node = node
                    .child_by_field_name("heading")
                    .unwrap()
                    .child_by_field_name("title")
                    .unwrap();
                let title = title_node.utf8_text(text).unwrap();
                Some(title.to_string())
            }
            // TODO: add more symbols. (if final syntax has more than headings)
            _ => None,
        }
        .map(|name| {
            let range = node.range().to_lsp_range();
            #[allow(deprecated)]
            let sym = lsp_types::DocumentSymbol {
                name,
                detail: Some(node.utf8_text(text).unwrap().to_string()),
                kind: lsp_types::SymbolKind::STRUCT,
                tags: None,
                range,
                selection_range: range,
                children: if cursor.goto_first_child() {
                    let children = tree_to_symbols(cursor, text);
                    if children.len() > 0 {
                        Some(children)
                    } else {
                        None
                    }
                } else {
                    None
                },
                deprecated: None,
            };
            symbols.push(sym);
        });
    }
    if cursor.goto_next_sibling() {
        symbols.append(&mut tree_to_symbols(cursor, text));
    } else {
        cursor.goto_parent();
    }
    return symbols;
}
