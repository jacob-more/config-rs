use std::{
    fmt::{Debug, Display},
    sync::LazyLock,
};

use lex_token_derive::lex;
use regex::bytes::Regex;

use crate::parse::{
    OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_ASSIGN_IF_UNDEFINED, OPERATOR_CLEAR, OPERATOR_GROUP,
    OPERATOR_REMOVE, OPERATOR_RESET,
};

#[cfg(test)]
mod test;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos {
    pub line: usize,
    pub column: usize,
}
impl Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // +1s so the displayed lines and columns start at 1.
        write!(f, "{}:{}", self.line + 1, self.column + 1)
    }
}
impl Debug for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub start: Pos,
    pub end: Pos,
}
impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}
impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

/// For the most part, getting the position is only needed when outputting
/// errors and other debugging tasks. To keep the happy path cheap, the span &
/// position information is generated on-the-fly.
fn get_pos(bytes: &[u8]) -> Pos {
    // Using a regexes here is just to make sure unicode is handled correctly.
    static LINE_END: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?-u:\n)").expect("static regex must be valid"));
    static COLUMN_CHAR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?-su:.*)").expect("static regex must be valid"));

    let (line_count, start_current_line) = LINE_END
        .find_iter(bytes)
        .fold((0, 0), |(count, _), matched| (count + 1, matched.end()));
    let column_count = COLUMN_CHAR
        .find(&bytes[start_current_line..])
        .map(|matched| matched.len())
        .unwrap_or(0);
    Pos {
        line: line_count,
        column: column_count,
    }
}

lex! {
    pub enum Token {
        Whitespace = r"(?-u:\s|\r|\n)+",
        Value = r##""(?<qstring>(?s-u)[^"\\]*)""##
            | r##""(?<qestring>(?s-u)(?:[^"\\]|\\.)*)""##
            | r"(?<string>(?u-s)[A-Za-z0-9_./](?:[A-Za-z0-9_./\-:]*[A-Za-z0-9_./])?)"
            | r"(?<estring>(?s-u)(?:[A-Za-z0-9_./]|\\.)(?:(?:[A-Za-z0-9_./\-:]|\\.)*(?:[A-Za-z0-9_./]|\\.))?)",
        BinaryOp = OPERATOR_ASSIGN
            | OPERATOR_ASSIGN_IF_UNDEFINED
            | OPERATOR_ADD
            | OPERATOR_REMOVE
            | OPERATOR_GROUP,
        SuffixUnaryOp = OPERATOR_CLEAR | OPERATOR_RESET,
        GroupingOpen = r"\{",
        GroupingClose = r"\}",
        Terminator = r";",
        Comment = r"(?-su:#.*)",
        Unknown = _,
    }
}
