use tree_sitter::{Node, Tree};

use crate::range::Position;

pub fn classify_for_decl(tree: &Tree, position: Position) -> Option<Syntax> {
    let point = position.into();
    let root = tree.root_node();
    let node = root.named_descendant_for_point_range(point, point)?;
    classify_node(node)
}

pub fn classify_node(node: Node) -> Option<Syntax> {
    let syntax_fn = match node.kind() {
        "section" => Syntax::Section,
        "link" => Syntax::Link,
        "anchor" => {
            if node.child_by_field_name("target").is_some() {
                Syntax::AnchorDefinition
            } else {
                Syntax::AnchorReference
            }
        }
        _ => {
            return classify_node(node.parent()?);
        }
    };
    Some(syntax_fn(node))
}

#[derive(Debug, PartialEq)]
pub enum Syntax<'a> {
    Section(Node<'a>),
    Link(Node<'a>),
    AnchorDefinition(Node<'a>),
    AnchorReference(Node<'a>),
}

impl<'a> Syntax<'a> {
    pub fn node(&self) -> &Node<'a> {
        match self {
            Self::Section(node)
            | Self::Link(node)
            | Self::AnchorDefinition(node)
            | Self::AnchorReference(node) => node,
        }
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
            Some(Syntax::Section(..)),
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
