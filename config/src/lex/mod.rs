use std::{
    fmt::{Debug, Display},
    iter::Peekable,
    ops::Deref,
    sync::LazyLock,
};

use bytes::Bytes;
use paste::paste;
use regex::{
    self,
    bytes::{CaptureMatches, Captures, Regex},
};

use crate::{
    ast::{
        OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_ASSIGN_IF_UNDEFINED, OPERATOR_CLEAR,
        OPERATOR_GROUP, OPERATOR_REMOVE, OPERATOR_RESET,
    },
    ext::IterJoin,
};

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

macro_rules! patterns {
    (@inner $($ident:ident),+ $(,)?) => {
        paste! {
            pub enum Token<'a> {
                $( $ident([<Token $ident>]<'a>), )+
            }
            impl<'a> Debug for Token<'a> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        $(
                            Self::$ident(token) => {
                                f.debug_struct(stringify!($ident))
                                    .field("buffer", &token.as_bytes())
                                    .field("span", &self.span())
                                    .finish()
                            },
                        )+
                    }
                }
            }
            impl<'a> Token<'a> {
                pub(crate) fn ident(&self) -> &'static str {
                    match self {
                        $( Self::$ident(_) => stringify!($ident), )+
                    }
                }

                pub fn as_slice(&self) -> &[u8] {
                    match self {
                        $( Self::$ident(token) => token.as_slice(), )+
                    }
                }

                pub fn as_bytes(&self) -> Bytes {
                    match self {
                        $( Self::$ident(token) => token.as_bytes(), )+
                    }
                }

                pub fn span(&self) -> Span {
                    match self {
                        $( Self::$ident(token) => token.span(), )+
                    }
                }

                pub fn start(&self) -> Pos {
                    match self {
                        $( Self::$ident(token) => token.start(), )+
                    }
                }

                pub fn end(&self) -> Pos {
                    match self {
                        $( Self::$ident(token) => token.end(), )+
                    }
                }
            }
        }
    };
    ($(($ident:ident, $capture:expr)),+ $(,)?) => {
        patterns!(@inner $($ident,)+ Unknown);

        paste! {
            $(
                pub struct [<Token $ident>]<'a> {
                    buffer: &'a Bytes,
                    captures: Captures<'a>,
                }
                impl<'a> Debug for [<Token $ident>]<'a> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.debug_struct(stringify!([<Token $ident>]))
                            .field("buffer", &self.as_bytes())
                            .field("span", &self.span())
                            .finish()
                    }
                }
                impl<'a> [<Token $ident>]<'a> {
                    pub fn as_slice(&self) -> &[u8] {
                        let matched = self.captures.get_match();
                        &self.buffer[matched.start()..matched.end()]
                    }

                    pub fn as_bytes(&self) -> Bytes {
                        let matched = self.captures.get_match();
                        self.buffer.slice(matched.start()..matched.end())
                    }

                    pub fn span(&self) -> Span {
                        let matched = self.captures.get_match();
                        let start = get_pos(&self.buffer[..matched.start()]);
                        let mut end = get_pos(&self.buffer[matched.start()..matched.end()]);
                        end.line += start.line;
                        if (start.line == end.line) {
                            end.column += start.column;
                        }
                        Span { start, end }
                    }

                    pub fn start(&self) -> Pos {
                        get_pos(&self.buffer[..self.captures.get_match().start()])
                    }

                    pub fn end(&self) -> Pos {
                        get_pos(&self.buffer[..self.captures.get_match().end()])
                    }
                }
            )+

            pub struct TokenUnknown<'a> {
                buffer: &'a Bytes,
                start: usize,
                end: usize,
            }
            impl<'a> Debug for TokenUnknown<'a> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct("TokenUnknown")
                        .field("buffer", &self.as_bytes())
                        .field("span", &self.span())
                        .finish()
                }
            }
            impl<'a> TokenUnknown<'a> {
                pub fn as_slice(&self) -> &[u8] {
                    &self.buffer[self.start..self.end]
                }

                pub fn as_bytes(&self) -> Bytes {
                    self.buffer.slice(self.start..self.end)
                }

                pub fn span(&self) -> Span {
                    let start = get_pos(&self.buffer[..self.start]);
                    let mut end = get_pos(&self.buffer[self.start..self.end]);
                    end.line += start.line;
                    if (start.line == end.line) {
                        end.column += start.column;
                    }
                    Span { start, end }
                }

                pub fn start(&self) -> Pos {
                    get_pos(&self.buffer[..self.start])
                }

                pub fn end(&self) -> Pos {
                    get_pos(&self.buffer[..self.end])
                }
            }

            $( const [<CAPTURE_NAME_ $ident:snake:upper>]: &'static str = $capture; )+

            #[derive(Debug, Clone)]
            pub struct TokenizerBuilder<'a> {
                $( [<$ident:snake:lower>]: Option<&'a str> ),+
            }

            impl<'a> Default for TokenizerBuilder<'a> {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl<'a> TokenizerBuilder<'a> {
                pub fn new() -> Self {
                    Self {
                        $( [<$ident:snake:lower>]: None ),+
                    }
                }

                $(
                    pub fn [<$ident:snake:lower>](&mut self, pattern: &'a str) {
                        self.[<$ident:snake:lower>] = Some(pattern);
                    }
                )+

                pub fn finalize(&self) -> Result<Tokenizer, regex::Error> {
                    let pattern = [
                        $(
                            (
                                [<CAPTURE_NAME_ $ident:snake:upper>],
                                self.[<$ident:snake:lower>],
                            ),
                        )+
                    ]
                    .into_iter()
                    .filter_map(|(name, pattern)| Some((name, pattern?)))
                    .map(|(name, pattern)| std::fmt::from_fn(move |f| {
                        write!(f, "(?<{name}>{pattern})")
                    }))
                    .join('|')
                    .to_string();
                    Ok(Tokenizer {
                        pattern: Regex::new(&pattern)?,
                        $( [<$ident:snake:lower>]: self.[<$ident:snake:lower>].is_some() ),+
                    })
                }
            }

            #[derive(Debug, Clone)]
            pub struct Tokenizer {
                pattern: Regex,
                $( [<$ident:snake:lower>]: bool ),+
            }

            impl Tokenizer {
                pub fn tokenize<'r, 'h>(&'r self, buffer: &'h Bytes) -> TokenIter<'r, 'h> {
                    TokenIter::new(self, buffer)
                }
            }

            #[derive(Debug)]
            pub struct TokenIter<'r, 'h> {
                buffer: &'h Bytes,
                tokenizer: &'r Tokenizer,
                last_end: usize,
                captures: Peekable<CaptureMatches<'r, 'h>>,
            }

            impl<'r, 'h> TokenIter<'r, 'h> {
                pub fn new(tokenizer: &'r Tokenizer, buffer: &'h Bytes) -> Self {
                    Self {
                        buffer,
                        tokenizer,
                        last_end: 0,
                        captures: tokenizer.pattern
                            .captures_iter(buffer.deref())
                            .peekable(),
                    }
                }
            }

            impl<'r, 'h> Iterator for TokenIter<'r, 'h> {
                type Item = Token<'h>;

                fn next(&mut self) -> Option<Self::Item> {
                    let captures = self.captures.peek()?;
                    if self.last_end < captures.get_match().start() {
                        let result = Some(Token::Unknown(TokenUnknown {
                            buffer: self.buffer,
                            start: self.last_end,
                            end: captures.get_match().start(),
                        }));
                        self.last_end = captures.get_match().start();
                        return result;
                    }
                    let captures = self.captures
                        .next()
                        .expect("peek operation succeeded. At least one value remained");
                    self.last_end = captures.get_match().end();
                    $(
                        if self.tokenizer.[<$ident:snake:lower>]
                            && captures.name([<CAPTURE_NAME_ $ident:snake:upper>]).is_some()
                        {
                            return Some(Token::$ident([<Token $ident>] {
                                buffer: self.buffer,
                                captures,
                            }))
                        }
                    )+
                    // TODO: verify if this is helpful
                    std::hint::cold_path();
                    panic!("must match at least one capture group")
                }
            }
        }
    };
}
patterns!(
    (SuffixUnaryOp, "sufuop"),
    (BinaryOp, "binop"),
    (GroupingOpen, "grpopn"),
    (GroupingClose, "grpcls"),
    (Terminator, "term"),
    (Comment, "cmt"),
    (Value, "val"),
    (Whitespace, "wtsp"),
);

pub static CONFIG_LEXICAL_TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
    let mut tokenizer = TokenizerBuilder::new();
    tokenizer.value(concat!(
        r##""(?<qestring>[^"\\]|\\.)*""##, // qstring + escapes
        r"|",
        r##""(?<qstring>[^"\\]*)""##, // qstring
        r"|",
        r"(?<estring>(?:[A-Za-z0-9_./]|\\.)(?:(?:[A-Za-z0-9_./\-:]|\\.)*(?:[A-Za-z0-9_./]|\\.))?)", // raw string + escapes
        r"|",
        r"(?<string>[A-Za-z0-9_./](?:[A-Za-z0-9_./\-:]*[A-Za-z0-9_./])?)", // raw string
    ));
    let suffix_unary_ops = [regex::escape(OPERATOR_RESET), regex::escape(OPERATOR_CLEAR)]
        .join('|')
        .to_string();
    tokenizer.suffix_unary_op(&suffix_unary_ops);
    let binary_ops = [
        regex::escape(OPERATOR_ASSIGN),
        regex::escape(OPERATOR_ASSIGN_IF_UNDEFINED),
        regex::escape(OPERATOR_ADD),
        regex::escape(OPERATOR_REMOVE),
        regex::escape(OPERATOR_GROUP),
    ]
    .join('|')
    .to_string();
    tokenizer.binary_op(&binary_ops);
    tokenizer.grouping_open(r"\{");
    tokenizer.grouping_close(r"\}");
    tokenizer.terminator(r";");
    tokenizer.comment(r"(?-su:#.*)");
    tokenizer.whitespace(r"(?-u:\s|\r|\n)+");
    tokenizer.finalize().unwrap()
});
