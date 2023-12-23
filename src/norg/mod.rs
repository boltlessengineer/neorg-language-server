use std::sync::OnceLock;

use lsp_types::CompletionItemKind;

#[derive(Debug)]
pub struct NorgCompletion {
    pub name: String,
    pub kind: CompletionItemKind,
    pub text: String,
    pub desc: String,
    // HACK: find better way to check validity
    /// valid parent node types
    pub valid_parents: Vec<String>,
}

macro_rules! cmp_item {
    ($name:expr, $kind:expr, $text:expr, $desc:expr) => {
        NorgCompletion {
            name: String::from($name),
            kind: $kind,
            text: String::from($text.trim()),
            desc: String::from($desc),
            valid_parents: vec![],
        }
    };
}

pub static NORG_BLOCKS: OnceLock<Vec<NorgCompletion>> = OnceLock::new();

pub fn init_norg_completion() {
    NORG_BLOCKS
        .set(vec![
            cmp_item!(
                "code",
                CompletionItemKind::SNIPPET,
                include_str!("./blocks/code_text.txt"),
                include_str!("./blocks/code_desc.md")
            ),
            // TODO: add more
        ])
        .unwrap();
}
