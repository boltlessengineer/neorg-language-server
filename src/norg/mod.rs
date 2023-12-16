use std::sync::OnceLock;

use lsp_types::CompletionItemKind;

#[derive(Debug)]
pub struct NorgCompletion {
    pub name: String,
    pub kind: CompletionItemKind,
    pub text: String,
    pub desc: String,
    /// valid parent node types
    pub valid_parents: Vec<String>,
}

impl Into<NorgCompletion> for (&str, CompletionItemKind, &str, &str) {
    fn into(self) -> NorgCompletion {
        NorgCompletion {
            name: String::from(self.0),
            kind: self.1,
            text: String::from(self.2.trim()),
            desc: String::from(self.3),
            valid_parents: vec![],
        }
    }
}

pub static NORG_BLOCKS: OnceLock<Vec<NorgCompletion>> = OnceLock::new();

// "@" -> all types of verbatim ranged tags
// "|" -> all types of standard ranged tags
// "#" -> all types of strong carryover tags

pub fn init_norg_completion() {
    NORG_BLOCKS
        .set(vec![
            (
                "code",
                CompletionItemKind::SNIPPET,
                include_str!("./blocks/code_text.txt"),
                include_str!("./blocks/code_desc.md"),
            )
                .into(),
            // (
            //     "code",
            //     include_str!("./blocks/code_text.txt"),
            //     include_str!("./blocks/code_desc.md"),
            // )
            //     .into(),
        ])
        .unwrap();
}
