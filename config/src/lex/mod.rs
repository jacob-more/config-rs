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

use crate::ext::IterJoin;

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
    ($(($ident:ident, $capture:expr)),+ $(,)?) => {
        pub enum Token<'a> {
            $( $ident(&'a Bytes, Captures<'a>), )+
            Unknown {
                buffer: &'a Bytes,
                start: usize,
                end: usize,
            },
        }

        impl<'a> Debug for Token<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$ident(buffer, captures) => {
                            f.debug_struct(stringify!($ident))
                                .field("buffer", &buffer.slice(captures.get_match().range()))
                                .field("span", &self.span())
                                .finish()
                        },
                    )+
                    Self::Unknown { buffer, start, end } => {
                        f.debug_struct("Unknown")
                                .field("buffer", &buffer.slice(start..end))
                                .field("span", &self.span())
                                .finish()
                    },
                }
            }
        }

        impl<'a> Token<'a> {
            pub fn span(&self) -> Span {
                let (start, mut end) = match self {
                    $(
                        Self::$ident(buffer, captures) => {
                            let matched = captures.get_match();
                            (
                                get_pos(&buffer[..matched.start()]),
                                get_pos(&buffer[matched.start()..matched.end()])
                            )
                        },
                    )+
                    Self::Unknown { buffer, start, end } => {
                        (
                            get_pos(&buffer[..*start]),
                            get_pos(&buffer[*start..*end])
                        )
                    },
                };
                end.line += start.line;
                if (start.line == end.line) {
                    end.column += start.column;
                }
                Span { start, end }
            }

            pub fn start(&self) -> Pos {
                match self {
                    $(
                        Self::$ident(buffer, captures) => get_pos(&buffer[..captures.get_match().start()]),
                    )+
                    Self::Unknown { buffer, start, end: _ } => get_pos(&buffer[..*start]),
                }
            }

            pub fn end(&self) -> Pos {
                match self {
                    $(
                        Self::$ident(buffer, captures) => get_pos(&buffer[..captures.get_match().end()]),
                    )+
                    Self::Unknown { buffer, start: _, end } => get_pos(&buffer[..*end]),
                }
            }
        }

        paste! {
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
                    let capture = self.captures.peek()?;
                    if self.last_end < capture.get_match().start() {
                        let result = Some(Token::Unknown {
                            buffer: self.buffer,
                            start: self.last_end,
                            end: capture.get_match().start(),
                        });
                        self.last_end = capture.get_match().start();
                        return result;
                    }
                    let capture = self.captures
                        .next()
                        .expect("peek operation succeeded. At least one value remained");
                    self.last_end = capture.get_match().end();
                    $(
                        if self.tokenizer.[<$ident:snake:lower>]
                            && capture.name([<CAPTURE_NAME_ $ident:snake:upper>]).is_some()
                        {
                            return Some(Token::$ident(self.buffer, capture))
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
    (Keyword, "kwrd"),
    (Identifier, "ident"),
    (PrefixUnaryOp, "preuop"),
    (SuffixUnaryOp, "sufuop"),
    (BinaryOp, "binop"),
    (Separator, "sep"),
    (GroupingOpen, "grpopn"),
    (GroupingClose, "grpcls"),
    (Terminator, "term"),
    (Comment, "cmt"),
    (Value, "val"),
    (Whitespace, "wtsp"),
);
