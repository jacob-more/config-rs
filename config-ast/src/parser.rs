use std::{
    fmt::{Display, from_fn},
    sync::LazyLock,
};

use bytes::Bytes;
use regex::bytes::{Match, Regex};
use thiserror::Error;

use crate::{AstEntry, AstTree, ImplAstEntry};

const CAPTURE_GB_GROUP: &str = "grp";
const CAPTURE_GB_BODY_OPEN: &str = "gbo";
const CAPTURE_GB_BODY_CLOSE: &str = "gbc";

const CAPTURE_KVP_KEY: &str = "key";
const CAPTURE_KVP_OPERATOR: &str = "op";
const CAPTURE_KVP_VALUE: &str = "val";

const CAPTURE_WHITESPACE: &str = "wsp";

#[derive(Debug, Clone)]
pub struct AstParser {
    regex: Regex,
}

pub struct AstParse {
    parser: AstParser,
    buffer: Bytes,
}

enum PrivateAstEntry {
    GroupOpen {
        name: Bytes,
        body_open: Bytes,
    },
    GroupClose {
        group_close: Bytes,
    },
    KeyOpValue {
        key: Bytes,
        operator: Bytes,
        value: Bytes,
    },
    Eof,
}

#[derive(Debug, Error)]
pub enum AstParseError {
    #[error("")]
    EarlyEndOfStream,
    #[error("")]
    BadGroup {
        context: Bytes,
        identifier: Bytes,
        body_open: Bytes,
        body_close: Bytes,
    },
    #[error("")]
    BadKeyOpValue {
        context: Bytes,
        identifier: Bytes,
        operator: Bytes,
        value: Bytes,
    },
    #[error("")]
    UnmatchedGroupClose { group_close: Bytes },
    #[error("")]
    UnknownSequence(Bytes),
}

impl AstParser {
    pub fn new() -> Self {
        static PARSE_PATERN: LazyLock<Regex> = LazyLock::new(|| {
            const IDENTIFIER: &str = r"(?:[A-Za-z0-9_]+)";
            const KVP_OPERATORS: &str = r"(?:=|\+=|-=|!=)";
            const TYPE_STRING: &str = r#"(?:"(?:[^"\\]|\\.)*")"#;
            const TYPE_BOOL: &str = r"(?:true|false)";
            const TYPE_INTEGER: &str = r"(?:[0-9]+)";
            const TYPE_FLOAT: &str = r"(?:[0-9]+\.[0-9]+)";
            const WHITESPACE: &str = r"(?:\s|\r\n|\n)";
            let catpure_group_open = from_fn(|f| {
                write!(f, r"(?<{CAPTURE_GB_GROUP}>{IDENTIFIER})")?;
                write!(f, r"{WHITESPACE}*")?;
                write!(f, r":")?;
                write!(f, r"{WHITESPACE}*")?;
                write!(f, r"(?<{CAPTURE_GB_BODY_OPEN}>\{{)")
            });
            let catpure_group_close = from_fn(|f| write!(f, r"(?<CAPTURE_GB_BODY_CLOSE>\}})"));
            let capture_key_value_pair = from_fn(|f| {
                write!(f, r"(?<{CAPTURE_KVP_KEY}>{IDENTIFIER})")?;
                write!(f, r"{WHITESPACE}*")?;
                write!(f, r"(?<{CAPTURE_KVP_OPERATOR}>{KVP_OPERATORS})")?;
                write!(f, r"{WHITESPACE}*")?;
                write!(
                    f,
                    r"(?<{CAPTURE_KVP_VALUE}>{TYPE_STRING}|{TYPE_BOOL}|{TYPE_INTEGER}|{TYPE_FLOAT})"
                )
            });
            let capture_whitespace =
                from_fn(|f| write!(f, "(?<{CAPTURE_WHITESPACE}>{WHITESPACE}*)"));
            let parse_pattern = from_fn(|f| {
                write!(f, r"{WHITESPACE}*")?;
                write!(
                    f,
                    r"{capture_key_value_pair}|{catpure_group_open}|{catpure_group_close}|{capture_whitespace}"
                )?;
                write!(f, r"{WHITESPACE}*")
            });
            Regex::new(&format!(r"(?s-u:{parse_pattern})")).unwrap()
        });

        Self {
            regex: PARSE_PATERN.clone(),
        }
    }
}

impl AstParser {
    pub fn parse_reader<R>(self, mut reader: R) -> std::io::Result<AstParse>
    where
        R: std::io::Read,
    {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(AstParse {
            parser: self,
            buffer: Bytes::from(buffer),
        })
    }

    pub fn parse_bytes<B>(self, bytes: B) -> AstParse
    where
        Bytes: From<B>,
    {
        AstParse {
            parser: self,
            buffer: bytes.into(),
        }
    }
}

impl AstParse {
    pub fn to_tree(self) -> Result<AstTree, AstParseError> {
        self.collect()
    }

    fn parse_next_entry(&mut self) -> Result<PrivateAstEntry, AstParseError> {
        if self.buffer.is_empty() {
            return Err(AstParseError::EarlyEndOfStream);
        }
        let search_buffer = self.buffer.clone();
        let captures = self
            .parser
            .regex
            .captures_at(&search_buffer, 0)
            .ok_or_else(|| AstParseError::UnknownSequence(search_buffer.clone()))?;
        let full_match = captures.get_match();
        debug_assert_eq!(
            full_match.start(),
            0,
            "AST entry match must start from beginning of buffer"
        );

        fn capture(buffer: &Bytes, regex_match: Option<Match<'_>>) -> Bytes {
            regex_match
                .map(|regex_match| buffer.slice_ref(regex_match.as_bytes()))
                .unwrap_or_default()
        }

        match (
            captures.name(CAPTURE_KVP_KEY),
            captures.name(CAPTURE_KVP_OPERATOR),
            captures.name(CAPTURE_KVP_VALUE),
        ) {
            (None, None, None) => (),
            (Some(key), Some(op), Some(value)) => {
                let buffer = self.buffer.split_to(full_match.end());
                return Ok(PrivateAstEntry::KeyOpValue {
                    key: buffer.slice_ref(key.as_bytes()),
                    operator: buffer.slice_ref(op.as_bytes()),
                    value: buffer.slice_ref(value.as_bytes()),
                });
            }
            (key, op, value) => {
                return Err(AstParseError::BadKeyOpValue {
                    context: self.buffer.slice_ref(full_match.as_bytes()),
                    identifier: capture(&self.buffer, key),
                    operator: capture(&self.buffer, op),
                    value: capture(&self.buffer, value),
                });
            }
        }

        match (
            captures.name(CAPTURE_GB_GROUP),
            captures.name(CAPTURE_GB_BODY_OPEN),
        ) {
            (None, None) => (),
            (Some(group), Some(body_open)) => {
                let buffer = self.buffer.split_to(full_match.end());
                return Ok(PrivateAstEntry::GroupOpen {
                    name: buffer.slice_ref(group.as_bytes()),
                    body_open: buffer.slice_ref(body_open.as_bytes()),
                });
            }
            (group, body_open) => {
                return Err(AstParseError::BadGroup {
                    context: self.buffer.slice_ref(full_match.as_bytes()),
                    identifier: capture(&self.buffer, group),
                    body_open: capture(&self.buffer, body_open),
                    body_close: Bytes::new(),
                });
            }
        }

        if let Some(body_close) = captures.name(CAPTURE_GB_BODY_CLOSE) {
            let buffer = self.buffer.split_to(full_match.end());
            return Ok(PrivateAstEntry::GroupClose {
                group_close: buffer.slice_ref(body_close.as_bytes()),
            });
        }

        panic!("parser is made up of capture groups")
    }

    fn parse_group_body(&mut self) -> Result<Vec<AstEntry>, AstParseError> {
        let mut body = Vec::new();
        loop {
            match self.parse_next_entry()? {
                PrivateAstEntry::GroupOpen { name, body_open: _ } => {
                    body.push(AstEntry(ImplAstEntry::Group {
                        name,
                        entries: self.parse_group_body()?,
                    }));
                }
                PrivateAstEntry::GroupClose { group_close: _ } => {
                    break;
                }
                PrivateAstEntry::KeyOpValue {
                    key,
                    operator,
                    value,
                } => {
                    body.push(AstEntry(ImplAstEntry::KeyOpValue {
                        key,
                        operator,
                        value,
                    }));
                }
                PrivateAstEntry::Eof => return Err(AstParseError::EarlyEndOfStream),
            }
        }
        Ok(body)
    }
}

impl Iterator for AstParse {
    type Item = Result<AstEntry, AstParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let context = self.buffer.clone();
        match self.parse_next_entry() {
            Ok(PrivateAstEntry::GroupOpen { name, body_open }) => match self.parse_group_body() {
                Ok(entries) => Some(Ok(AstEntry(ImplAstEntry::Group { name, entries }))),
                Err(AstParseError::EarlyEndOfStream) => Some(Err(AstParseError::BadGroup {
                    context,
                    identifier: name,
                    body_open,
                    body_close: Bytes::new(),
                })),
                Err(error) => Some(Err(error)),
            },
            Ok(PrivateAstEntry::GroupClose { group_close }) => {
                Some(Err(AstParseError::UnmatchedGroupClose { group_close }))
            }
            Ok(PrivateAstEntry::KeyOpValue {
                key,
                operator,
                value,
            }) => Some(Ok(AstEntry(ImplAstEntry::KeyOpValue {
                key,
                operator,
                value,
            }))),
            Ok(PrivateAstEntry::Eof) => None,
            Err(error) => Some(Err(error)),
        }
    }
}

// struct JoinSlice<'a, T, S> {
//     slice: &'a [T],
//     separator: S,
// }

// impl<'a, T, S> JoinSlice<'a, T, S> {
//     pub fn new(slice: &'a [T], separator: S) -> Self {
//         Self { slice, separator }
//     }
// }

// impl<'a, T, S> Display for JoinSlice<'a, T, S>
// where
//     T: Display,
//     S: Display,
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         if let Some((first, tail)) = self.slice.split_first() {
//             write!(f, "{first}")?;
//             for value in tail {
//                 write!(f, "{}{value}", self.separator)?;
//             }
//         }
//         Ok(())
//     }
// }

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::{AstTree, parser::AstParser};

    #[rstest]
    #[case("", AstTree { entries: vec![] })]
    #[case("\n", AstTree { entries: vec![] })]
    #[case("\r\n", AstTree { entries: vec![] })]
    #[case("\t", AstTree { entries: vec![] })]
    #[case(" ", AstTree { entries: vec![] })]
    #[case("\n\r\n\t ", AstTree { entries: vec![] })]
    fn parse_str_ok(#[case] input: &str, #[case] output: AstTree) {
        let input_bytes = input.as_bytes().to_vec();
        let ast = AstParser::new().parse_bytes(input_bytes).to_tree();
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), output);
    }
}
