//! Abstract Syntax Tree definitions for the Arca programming language.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub start_loc: Location,
    pub end_loc: Location,
}

impl Span {
    pub fn new(start: usize, end: usize, start_loc: Location, end_loc: Location) -> Self {
        Self {
            start,
            end,
            start_loc,
            end_loc,
        }
    }
}
