use std::{ffi::OsStr, fmt::from_fn, os::unix::ffi::OsStrExt, sync::LazyLock};

use bytes::Bytes;
use regex::bytes::{Captures, Match, Regex};
use thiserror::Error;

use crate::{
    ast::{
        AstEntry, AstTree, OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_ASSIGN_IF_UNDEFINED,
        OPERATOR_CLEAR, OPERATOR_REMOVE, OPERATOR_RESET,
    },
    ext::{IterEscaped, IterJoin},
};

#[cfg(test)]
mod property_test;
#[cfg(test)]
mod test;

const CAPTURE_ESCAPE_KEY: &str = "ekey";
const CAPTURE_QUOTED_KEY: &str = "qkey";
const CAPTURE_UNQUOTED_KEY: &str = "ukey";

const CAPTURE_GB_BODY_OPEN: &str = "gbo";
const CAPTURE_GB_BODY_CLOSE: &str = "gbc";

const CAPTURE_KVP_ASSIGN_OPERATOR: &str = "aop";
const CAPTURE_KVP_RESET_OPERATOR: &str = "rop";
const CAPTURE_KVP_VALUE_ESCAPE: &str = "eval";
const CAPTURE_KVP_VALUE_QUOTED: &str = "qval";
const CAPTURE_KVP_VALUE_UNQUOTED: &str = "uval";

const CAPTURE_WHITESPACE: &str = "wsp";

const OPERATOR_BYTES_ASSIGN: &[u8] = OPERATOR_ASSIGN.as_bytes();
const OPERATOR_BYTES_ASSIGN_IF_UNDEFINED: &[u8] = OPERATOR_ASSIGN_IF_UNDEFINED.as_bytes();
const OPERATOR_BYTES_ADD: &[u8] = OPERATOR_ADD.as_bytes();
const OPERATOR_BYTES_REMOVE: &[u8] = OPERATOR_REMOVE.as_bytes();
const OPERATOR_BYTES_RESET: &[u8] = OPERATOR_RESET.as_bytes();
const OPERATOR_BYTES_CLEAR: &[u8] = OPERATOR_CLEAR.as_bytes();

#[derive(Debug, Clone)]
pub struct AstParser {
    regex: Regex,
}

pub struct AstParse {
    parser: AstParser,
    buffer: Bytes,
}

#[derive(Debug, Error)]
pub enum AstParseError {
    #[error("group {} on line {line} is missing closing brace\n{}", OsStr::from_bytes(identifier).display(), OsStr::from_bytes(context).display())]
    IncompleteGroup {
        line: usize,
        context: Bytes,
        identifier: Bytes,
    },
    #[error("unmatched closing brace on line {line}\n{}", OsStr::from_bytes(group_close).display())]
    UnmatchedGroupClose { line: usize, group_close: Bytes },
    #[error("unknown sequence on line {line}\n{}", OsStr::from_bytes(sequence).display())]
    UnknownSequence { line: usize, sequence: Bytes },
}

impl Default for AstParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AstParser {
    pub fn new() -> Self {
        static PARSE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            const TYPE_ESCAPED_STRING: &str = r#"(?:[^"\\]|\\.)*"#;
            const TYPE_QUOTED_STRING: &str = r#"[^"\\]*"#;
            const TYPE_UNQUOTED_STRING: &str =
                r"(?:[A-Za-z0-9_./](?:[A-Za-z0-9_./\-:]*[A-Za-z0-9_./])?)";
            const WHITESPACE: &str = r"(?:\s|\r\n|\n)";
            const COMMENT: &str = r"(?:\s*#[^\n]*)";
            let assign_operators = from_fn(|f| {
                write!(
                    f,
                    r"(?:{})",
                    [
                        regex::escape(OPERATOR_ASSIGN),
                        regex::escape(OPERATOR_ASSIGN_IF_UNDEFINED),
                        regex::escape(OPERATOR_ADD),
                        regex::escape(OPERATOR_REMOVE),
                    ]
                    .join('|'),
                )
            });
            let reset_operators = from_fn(|f| {
                write!(
                    f,
                    r"(?:{})",
                    [regex::escape(OPERATOR_CLEAR), regex::escape(OPERATOR_RESET)].join('|'),
                )
            });
            let capture_group_open =
                from_fn(|f| write!(f, r"(?<{CAPTURE_GB_BODY_OPEN}>:{WHITESPACE}*\{{)"));
            let capture_group_close = from_fn(|f| write!(f, r"(?<{CAPTURE_GB_BODY_CLOSE}>\}})"));
            let capture_assignment_op_value = from_fn(|f| {
                write!(f, r"(?<{CAPTURE_KVP_ASSIGN_OPERATOR}>{assign_operators})")?;
                write!(f, r"{WHITESPACE}*")?;

                write!(f, r"(?:")?;
                write!(
                    f,
                    r#""(?<{CAPTURE_KVP_VALUE_QUOTED}>{TYPE_QUOTED_STRING})""#
                )?;
                write!(f, r"|")?;
                write!(
                    f,
                    r#""(?<{CAPTURE_KVP_VALUE_ESCAPE}>{TYPE_ESCAPED_STRING})""#
                )?;
                write!(f, r"|")?;
                write!(
                    f,
                    r"(?<{CAPTURE_KVP_VALUE_UNQUOTED}>{TYPE_UNQUOTED_STRING})"
                )?;
                write!(f, r")")
            });
            let capture_reset_op =
                from_fn(|f| write!(f, r"(?<{CAPTURE_KVP_RESET_OPERATOR}>{reset_operators})"));
            let capture_key_operator_value = from_fn(|f| {
                write!(f, r"(?:")?;
                write!(f, r#""(?<{CAPTURE_QUOTED_KEY}>{TYPE_QUOTED_STRING})""#)?;
                write!(f, r"|")?;
                write!(f, r#""(?<{CAPTURE_ESCAPE_KEY}>{TYPE_ESCAPED_STRING})""#)?;
                write!(f, r"|")?;
                write!(f, r"(?<{CAPTURE_UNQUOTED_KEY}>{TYPE_UNQUOTED_STRING})")?;
                write!(f, r")")?;
                write!(f, r"{WHITESPACE}*")?;
                write!(
                    f,
                    r"(?:{capture_group_open}|{capture_assignment_op_value}|{capture_reset_op})"
                )
            });
            let capture_whitespace =
                from_fn(|f| write!(f, "(?<{CAPTURE_WHITESPACE}>{COMMENT}|{WHITESPACE}+)"));
            let parse_pattern = from_fn(|f| {
                write!(f, r"{WHITESPACE}*")?;
                write!(f, r"(?:")?;
                write!(f, r"{capture_key_operator_value}")?;
                write!(f, r"|{capture_group_close}")?;
                write!(f, r"|{capture_whitespace}")?;
                write!(f, r")")?;
                // A semi-colon can be used as an optional terminator.
                write!(f, r"(?:{WHITESPACE}*;)?")
            });
            Regex::new(&format!(r"(?s-u:{parse_pattern})")).unwrap()
        });

        Self {
            regex: PARSE_PATTERN.clone(),
        }
    }
}

fn count_lines(bytes: &[u8]) -> usize {
    // Using a regex here is just to make sure unicode is handled correctly.
    static LINE_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s-u:\n)").unwrap());
    LINE_END.find_iter(bytes).count() + 1
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
    pub fn parse_into_tree(self) -> Result<AstTree, Box<AstParseError>> {
        fn capture_string<'a>(
            source_buffer: &Bytes,
            captured: &Captures<'a>,
            key_unquoted: &str,
            key_quoted: &str,
            key_escaped: &str,
        ) -> Option<(Match<'a>, Bytes)> {
            captured
                .name(key_unquoted)
                .or_else(|| captured.name(key_quoted))
                .map(|matched| (matched, source_buffer.slice_ref(matched.as_bytes())))
                .or_else(|| {
                    // Replacing escaped characters requires allocating a new
                    // `Bytes` buffer. We'd rather not re-allocate. Hence, why
                    // this is its own capture group.
                    captured
                        .name(key_escaped)
                        .map(|matched| (matched, matched.as_bytes().unescaped().copied().collect()))
                })
        }

        struct AstGroup {
            span_start: usize,
            name: Bytes,
            entries: Vec<AstEntry>,
        }

        let mut stack = Vec::new();
        stack.push(AstGroup {
            span_start: 0,
            name: Bytes::new(),
            entries: Vec::new(),
        });
        let mut next_start = 0;
        for captured in self.parser.regex.captures_iter(&self.buffer) {
            let matched = captured.get_match();
            if matched.start() > next_start {
                return Err(Box::new(AstParseError::UnknownSequence {
                    line: count_lines(&self.buffer[..matched.start()]),
                    sequence: self.buffer.slice(next_start..matched.start()),
                }));
            }
            debug_assert_eq!(
                matched.start(),
                next_start,
                "match start cannot fall behind the start of the last match"
            );
            next_start = matched.end();

            // CAPTURE_QUOTED_KEY
            if let Some((matched_key, key)) = capture_string(
                &self.buffer,
                &captured,
                CAPTURE_UNQUOTED_KEY,
                CAPTURE_QUOTED_KEY,
                CAPTURE_ESCAPE_KEY,
            ) {
                let entry = if let Some(matched_op) = captured.name(CAPTURE_KVP_ASSIGN_OPERATOR) {
                    let value = capture_string(
                        &self.buffer,
                        &captured,
                        CAPTURE_KVP_VALUE_UNQUOTED,
                        CAPTURE_KVP_VALUE_QUOTED,
                        CAPTURE_KVP_VALUE_ESCAPE,
                    )
                    .map(|(_, value)| value)
                    .unwrap_or_default();
                    match matched_op.as_bytes() {
                        OPERATOR_BYTES_ASSIGN => AstEntry::new_assign(key, value),
                        OPERATOR_BYTES_ASSIGN_IF_UNDEFINED => {
                            AstEntry::new_assign_if_undefined(key, value)
                        }
                        OPERATOR_BYTES_ADD => AstEntry::new_add(key, value),
                        OPERATOR_BYTES_REMOVE => AstEntry::new_remove(key, value),
                        _ => panic!(
                            "operator should not match assign operators: {}",
                            OsStr::from_bytes(matched_op.as_bytes()).display()
                        ),
                    }
                } else if let Some(matched_op) = captured.name(CAPTURE_KVP_RESET_OPERATOR) {
                    match matched_op.as_bytes() {
                        OPERATOR_BYTES_CLEAR => AstEntry::new_clear(key),
                        OPERATOR_BYTES_RESET => AstEntry::new_reset(key),
                        _ => panic!(
                            "operator should not match reset operators: {}",
                            OsStr::from_bytes(matched_op.as_bytes()).display()
                        ),
                    }
                } else if captured.name(CAPTURE_GB_BODY_OPEN).is_some() {
                    stack.push(AstGroup {
                        span_start: matched_key.start(),
                        name: key,
                        entries: Vec::new(),
                    });
                    continue;
                } else {
                    panic!(
                        "key was found. Must match one of the operator options: assign ops, reset ops, or group open"
                    )
                };
                stack
                    .last_mut()
                    .expect("stack initialized with one element")
                    .entries
                    .push(entry);
                continue;
            }

            if let Some(matched_body_close) = captured.name(CAPTURE_GB_BODY_CLOSE) {
                let closed_group = stack.pop().expect("stack initialized with one element");
                match stack.last_mut() {
                    Some(append_to_group) => {
                        append_to_group
                            .entries
                            .push(AstEntry::new_group(closed_group.name, closed_group.entries));
                        continue;
                    }
                    None => {
                        return Err(Box::new(AstParseError::UnmatchedGroupClose {
                            line: count_lines(&self.buffer[..matched_body_close.end()]),
                            group_close: self.buffer.slice_ref(matched_body_close.as_bytes()),
                        }));
                    }
                }
            }

            if captured.name(CAPTURE_WHITESPACE).is_some() {
                continue;
            }

            panic!(
                "at least one capture group must be met but match '{}' met none",
                OsStr::from_bytes(matched.as_bytes()).display()
            );
        }

        if stack.len() > 1 {
            let incomplete_group = stack.pop().expect("stack len is greater than 1");
            Err(Box::new(AstParseError::IncompleteGroup {
                line: count_lines(&self.buffer[..incomplete_group.span_start]),
                context: self.buffer.slice(incomplete_group.span_start..),
                identifier: incomplete_group.name,
            }))
        } else {
            let tree = stack.pop().expect("stack initialized with one element");
            Ok(AstTree::from_iter(tree.entries))
        }
    }
}
