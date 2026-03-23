use std::{ffi::OsStr, fmt::from_fn, os::unix::ffi::OsStrExt, sync::LazyLock};

use bytes::Bytes;
use regex::bytes::Regex;
use thiserror::Error;

use crate::{AstEntry, AstTree};

const CAPTURE_GB_GROUP: &str = "grp";
const CAPTURE_GB_BODY_OPEN: &str = "gbo";
const CAPTURE_GB_BODY_CLOSE: &str = "gbc";

const CAPTURE_KVP_KEY: &str = "key";
const CAPTURE_KVP_ASSIGN_OPERATOR: &str = "aop";
const CAPTURE_KVP_RESET_OPERATOR: &str = "rop";
const CAPTURE_KVP_VALUE_QUOTED: &str = "qval";
const CAPTURE_KVP_VALUE_UNQUOTED: &str = "uval";

const CAPTURE_WHITESPACE: &str = "wsp";

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

impl AstParser {
    pub fn new() -> Self {
        static PARSE_PATERN: LazyLock<Regex> = LazyLock::new(|| {
            const IDENTIFIER: &str = r"(?:[A-Za-z0-9_]+)";
            const KVP_ASSIGN_OPERATORS: &str = r"(?:=|:=|\+=|-=)";
            const KVP_RESET_OPERATOR: &str = r"(?:!)";
            const TYPE_QUOTED_STRING: &str = r#"(?:[^"\\]|\\.)*"#;
            const TYPE_UNQUOTED_STRING: &str = r"(?:[A-Za-z0-9_./\-]+)";
            const WHITESPACE: &str = r"(?:\s|\r\n|\n)";
            let catpure_group_open = from_fn(|f| {
                write!(f, r"(?<{CAPTURE_GB_GROUP}>{IDENTIFIER})")?;
                write!(f, r"{WHITESPACE}*")?;
                write!(f, r":")?;
                write!(f, r"{WHITESPACE}*")?;
                write!(f, r"(?<{CAPTURE_GB_BODY_OPEN}>\{{)")
            });
            let catpure_group_close = from_fn(|f| write!(f, r"(?<{CAPTURE_GB_BODY_CLOSE}>\}})"));
            let capture_key_value_pair = from_fn(|f| {
                write!(f, r"(?<{CAPTURE_KVP_KEY}>{IDENTIFIER})")?;
                write!(f, r"{WHITESPACE}*")?;
                write!(f, r"(?:")?;
                write!(
                    f,
                    r"(?<{CAPTURE_KVP_ASSIGN_OPERATOR}>{KVP_ASSIGN_OPERATORS})"
                )?;
                write!(f, r"{WHITESPACE}*")?;
                write!(f, r"(?:")?;
                write!(
                    f,
                    r#""(?<{CAPTURE_KVP_VALUE_QUOTED}>{TYPE_QUOTED_STRING})""#
                )?;
                write!(f, r"|")?;
                write!(
                    f,
                    r"(?<{CAPTURE_KVP_VALUE_UNQUOTED}>{TYPE_UNQUOTED_STRING})"
                )?;
                write!(f, r")")?;
                write!(f, r"|")?;
                write!(f, r"(?<{CAPTURE_KVP_RESET_OPERATOR}>{KVP_RESET_OPERATOR})")?;
                write!(f, r")")
            });
            let capture_whitespace =
                from_fn(|f| write!(f, "(?<{CAPTURE_WHITESPACE}>{WHITESPACE}+)"));
            let parse_pattern = from_fn(|f| {
                write!(f, r"{WHITESPACE}*")?;
                write!(f, r"(?:")?;
                write!(f, r"{capture_key_value_pair}")?;
                write!(f, r"|{catpure_group_open}")?;
                write!(f, r"|{catpure_group_close}")?;
                write!(f, r"|{capture_whitespace}")?;
                write!(f, r")")?;
                // A semi-colon can be used as an optional terminator.
                write!(f, r"(?:{WHITESPACE}*;)?")
            });
            Regex::new(&format!(r"(?s-u:{parse_pattern})")).unwrap()
        });

        Self {
            regex: PARSE_PATERN.clone(),
        }
    }
}

fn count_lines(bytes: &[u8]) -> usize {
    // Using a regex here is just to make sure unicode is handled correctly.
    static LINE_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s-u:\n)").unwrap());
    LINE_END.find_iter(bytes).count()
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
                return Err(AstParseError::UnknownSequence {
                    line: count_lines(&self.buffer[..matched.start()]),
                    sequence: self.buffer.slice(next_start..matched.start()),
                });
            }
            debug_assert_eq!(
                matched.start(),
                next_start,
                "match start cannot fall behind the start of the last match"
            );
            next_start = matched.end();

            if let Some(key) = captured.name(CAPTURE_KVP_KEY) {
                let (op, value) = if let Some(op) = captured.name(CAPTURE_KVP_ASSIGN_OPERATOR) {
                    let value = if let Some(value) = captured.name(CAPTURE_KVP_VALUE_QUOTED) {
                        self.buffer.slice_ref(value.as_bytes())
                    } else if let Some(value) = captured.name(CAPTURE_KVP_VALUE_UNQUOTED) {
                        self.buffer.slice_ref(value.as_bytes())
                    } else {
                        Bytes::new()
                    };
                    (op, value)
                } else {
                    let op = captured
                        .name(CAPTURE_KVP_RESET_OPERATOR)
                        .expect("an operation must be present if key-op-value key is found");
                    (op, Bytes::new())
                };
                stack
                    .last_mut()
                    .expect("stack initialized with one element")
                    .entries
                    .push(AstEntry::new_key_value(
                        self.buffer.slice_ref(key.as_bytes()),
                        self.buffer.slice_ref(op.as_bytes()),
                        value,
                    ));
                continue;
            }

            if let Some(key) = captured.name(CAPTURE_GB_GROUP) {
                let _ = captured
                    .name(CAPTURE_GB_BODY_OPEN)
                    .expect("body-open must be present if group key is found");
                stack.push(AstGroup {
                    span_start: key.start(),
                    name: self.buffer.slice_ref(key.as_bytes()),
                    entries: Vec::new(),
                });
                continue;
            }

            if let Some(body_close) = captured.name(CAPTURE_GB_BODY_CLOSE) {
                let closed_group = stack.pop().expect("stack initialized with one element");
                match stack.last_mut() {
                    Some(append_to_group) => {
                        append_to_group
                            .entries
                            .push(AstEntry::new_group(closed_group.name, closed_group.entries));
                        continue;
                    }
                    None => {
                        return Err(AstParseError::UnmatchedGroupClose {
                            line: count_lines(&self.buffer[..body_close.end()]),
                            group_close: self.buffer.slice_ref(body_close.as_bytes()),
                        });
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
            Err(AstParseError::IncompleteGroup {
                line: count_lines(&self.buffer[..incomplete_group.span_start]),
                context: self.buffer.slice(incomplete_group.span_start..),
                identifier: incomplete_group.name,
            })
        } else {
            let tree = stack.pop().expect("stack initialized with one element");
            Ok(AstTree {
                entries: tree.entries,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::{AstEntry, AstTree, parser::AstParser};

    #[rstest]
    #[case(b"")]
    #[case(b" ")]
    #[case(b"\t")]
    #[case(b"\n")]
    #[case(b"\r\n")]
    #[case(b" \t\n\r\n")]
    fn parse_empty_ast(#[case] input: &[u8]) {
        let ast = AstParser::new().parse_bytes(input.to_vec()).to_tree();
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), AstTree::new());
    }

    #[rstest]
    #[case(
        b"KEY=UNQUOTED_STRING",
        AstTree {
            entries: vec![
                AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY_WITH_UNDERSCORES=UNQUOTED_STRING/0123456789.",
        AstTree {
            entries: vec![
                AstEntry::new_assign(b"KEY_WITH_UNDERSCORES".to_vec(), b"UNQUOTED_STRING/0123456789.".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY=\"QUOTED String 0123456789 \\\\ \\\"\"",
        AstTree {
            entries: vec![
                AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String 0123456789 \\\\ \\\"".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY=UNQUOTED_STRING;",
        AstTree {
            entries: vec![
                AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY = UNQUOTED_STRING ;",
        AstTree {
            entries: vec![
                AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY\n=\n\t    UNQUOTED_STRING;",
        AstTree {
            entries: vec![
                AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY=UNQUOTED_STRING;KEY2=\"QUOTED String @\";",
        AstTree {
            entries: vec![
                AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
                AstEntry::new_assign(b"KEY2".to_vec(), b"QUOTED String @".to_vec()),
            ]
        }
    )]
    #[case(
        b"
        KEY+=UNQUOTED_STRING
        KEY=\"QUOTED String @\";
        ",
        AstTree {
            entries: vec![
                AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
                AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
            ]
        }
    )]
    #[case(
        b"KEY+=UNQUOTED_STRING",
        AstTree {
            entries: vec![
                AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY-=UNQUOTED_STRING",
        AstTree {
            entries: vec![
                AstEntry::new_remove(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY:=UNQUOTED_STRING",
        AstTree {
            entries: vec![
                AstEntry::new_assign_if_undefined(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY!",
        AstTree {
            entries: vec![
                AstEntry::new_reset(b"KEY".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY !",
        AstTree {
            entries: vec![
                AstEntry::new_reset(b"KEY".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY !",
        AstTree {
            entries: vec![
                AstEntry::new_reset(b"KEY".to_vec())
            ]
        }
    )]
    #[case(
        b"KEY!
        KEY=\"QUOTED String @\";
        ",
        AstTree {
            entries: vec![
                AstEntry::new_reset(b"KEY".to_vec()),
                AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
            ]
        }
    )]
    #[case(
        b"KEY!NEXT-=VALUE",
        AstTree {
            entries: vec![
                AstEntry::new_reset(b"KEY".to_vec()),
                AstEntry::new_remove(b"NEXT".to_vec(), b"VALUE".to_vec()),
            ]
        }
    )]
    fn parse_key_op_value(#[case] input: &[u8], #[case] output: AstTree) {
        let ast = AstParser::new().parse_bytes(input.to_vec()).to_tree();
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), output);
    }
}
