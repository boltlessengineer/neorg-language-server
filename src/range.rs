#[derive(Clone, Debug)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}
impl Position {
    pub fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}
impl From<lsp_types::Position> for Position {
    fn from(pos: lsp_types::Position) -> Self {
        Self {
            row: pos.line as usize,
            column: pos.character as usize,
        }
    }
}
impl From<tree_sitter::Point> for Position {
    fn from(pos: tree_sitter::Point) -> Self {
        Self {
            row: pos.row,
            column: pos.column,
        }
    }
}
impl From<(usize, usize)> for Position {
    fn from(pos: (usize, usize)) -> Self {
        Self {
            row: pos.0,
            column: pos.1,
        }
    }
}
impl Into<tree_sitter::Point> for Position {
    fn into(self) -> tree_sitter::Point {
        tree_sitter::Point {
            row: self.row,
            column: self.column,
        }
    }
}
impl Into<lsp_types::Position> for Position {
    fn into(self) -> lsp_types::Position {
        lsp_types::Position {
            line: self.row as u32,
            character: self.column as u32,
        }
    }
}
#[derive(Clone)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}
impl From<Position> for Range {
    fn from(pos: Position) -> Self {
        Self { start: pos.clone(), end: pos }
    }
}
impl From<tree_sitter::Range> for Range {
    fn from(range: tree_sitter::Range) -> Self {
        Self {
            start: range.start_point.into(),
            end: range.end_point.into(),
        }
    }
}
impl From<lsp_types::Range> for Range {
    fn from(range: lsp_types::Range) -> Self {
        Self {
            start: range.start.into(),
            end: range.end.into(),
        }
    }
}
impl Into<lsp_types::Range> for Range {
    fn into(self) -> lsp_types::Range {
        lsp_types::Range {
            start: self.start.into(),
            end: self.end.into(),
        }
    }
}
