use tree_sitter::{Node, Tree};

use crate::range::Position;

pub fn classify_for_decl(tree: &Tree, position: Position) -> Option<Syntax> {
    let point = position.into();
    let root = tree.root_node();
    let node = root.named_descendant_for_point_range(point, point)?;
    classify_node(node)
}

fn classify_node(node: Node) -> Option<Syntax> {
    let syntax_fn = match node.kind() {
        "heading" => Syntax::Heading,
        "link" | "unclosed_link" => Syntax::Link,
        "anchor" | "unclosed_anchor" => {
            if node.child_by_field_name("target").is_some() {
                Syntax::AnchorDefinition
            } else {
                Syntax::AnchorReference
            }
        }
        _ => {
            let parent = node.parent()?;
            match parent.kind() {
                "section" | "document" => {
                    // TODO: classify section_cnotent block as NormalBlock
                    return None;
                }
                _ => return classify_node(parent),
            }
        }
    };
    Some(syntax_fn(node))
}

#[derive(Debug, PartialEq)]
pub enum Syntax<'a> {
    Heading(Node<'a>),
    Link(Node<'a>),
    AnchorDefinition(Node<'a>),
    AnchorReference(Node<'a>),
    NormalBlock(Node<'a>),
}

#[derive(Debug, PartialEq)]
pub enum CompletionSyntax<'a> {
    Target(Node<'a>),
    AnchorIdent(Node<'a>),
}

impl<'a> CompletionSyntax<'a> {
    pub fn classify_from(node: Node<'a>) -> Option<Self> {
        let syntax_fn = match node.kind() {
            "target" | "target_unclosed" => Self::Target,
            "markup" | "markup_unclosed" => Self::AnchorIdent,
            _ => {
                let parent = node.parent()?;
                match parent.kind() {
                    "paragraph" => return None,
                    _ => return Self::classify_from(parent),
                }
            }
        };
        Some(syntax_fn(node))
    }
}

#[cfg(test)]
mod test {
    use crate::tree_sitter::parse_norg;

    use super::*;

    #[test]
    fn test_classify_for_decl() {
        let text = "* _heading_ with [anchor]\n{link}";
        let tree = parse_norg(&text, None).unwrap();
        assert!(matches!(
            classify_for_decl(&tree, Position::new(0, 3)),
            Some(Syntax::Heading(..)),
        ));
        assert!(matches!(
            classify_for_decl(&tree, Position::new(0, 17)),
            Some(Syntax::AnchorReference(..)),
        ));
        assert!(matches!(
            classify_for_decl(&tree, Position::new(0, 24)),
            Some(Syntax::AnchorReference(..)),
        ));
        assert!(matches!(
            classify_for_decl(&tree, Position::new(1, 0)),
            Some(Syntax::Link(..)),
        ));
    }
}
