use std::{ffi::OsStr, fmt::from_fn, os::unix::ffi::OsStrExt, sync::LazyLock};

use bytes::Bytes;
use regex::bytes::Regex;
use thiserror::Error;

use crate::{
    ast::{
        AstEntry, AstTree, OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_ASSIGN_IF_UNDEFINED,
        OPERATOR_REMOVE, OPERATOR_RESET,
    },
    ext::IterJoin,
};

const CAPTURE_GB_GROUP: &str = "grp";
const CAPTURE_GB_BODY_OPEN: &str = "gbo";
const CAPTURE_GB_BODY_CLOSE: &str = "gbc";

const CAPTURE_KVP_KEY: &str = "key";
const CAPTURE_KVP_ASSIGN_OPERATOR: &str = "aop";
const CAPTURE_KVP_RESET_OPERATOR: &str = "rop";
const CAPTURE_KVP_VALUE_QUOTED: &str = "qval";
const CAPTURE_KVP_VALUE_UNQUOTED: &str = "uval";

const CAPTURE_WHITESPACE: &str = "wsp";

const OPERATOR_BYTES_ASSIGN: &[u8] = OPERATOR_ASSIGN.as_bytes();
const OPERATOR_BYTES_ASSIGN_IF_UNDEFINED: &[u8] = OPERATOR_ASSIGN_IF_UNDEFINED.as_bytes();
const OPERATOR_BYTES_ADD: &[u8] = OPERATOR_ADD.as_bytes();
const OPERATOR_BYTES_REMOVE: &[u8] = OPERATOR_REMOVE.as_bytes();
const OPERATOR_BYTES_RESET: &[u8] = OPERATOR_RESET.as_bytes();

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
        static PARSE_PATERN: LazyLock<Regex> = LazyLock::new(|| {
            const IDENTIFIER: &str = r"(?:[A-Za-z0-9_.]+|[A-Za-z0-9_.][A-Za-z0-9_./\-:]*[A-Za-z0-9_.])";
            let kvp_assign_operators = from_fn(|f| {
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
            let kvp_reset_operators =
                from_fn(|f| write!(f, r"(?:{})", [regex::escape(OPERATOR_RESET)].join('|'),));
            const TYPE_QUOTED_STRING: &str = r#"(?:[^"\\]|\\.)*"#;
            const TYPE_UNQUOTED_STRING: &str = r"(?:[A-Za-z0-9_./\-:]+)";
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
                    r"(?<{CAPTURE_KVP_ASSIGN_OPERATOR}>{kvp_assign_operators})"
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
                write!(f, r"(?<{CAPTURE_KVP_RESET_OPERATOR}>{kvp_reset_operators})")?;
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
    pub fn parse_into_tree(self) -> Result<AstTree, AstParseError> {
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
                let key = self.buffer.slice_ref(key.as_bytes());
                let entry = if let Some(op) = captured.name(CAPTURE_KVP_ASSIGN_OPERATOR) {
                    let mut value = if let Some(value) = captured.name(CAPTURE_KVP_VALUE_QUOTED) {
                        self.buffer.slice_ref(value.as_bytes())
                    } else if let Some(value) = captured.name(CAPTURE_KVP_VALUE_UNQUOTED) {
                        self.buffer.slice_ref(value.as_bytes())
                    } else {
                        Bytes::new()
                    };
                    // Replacing escaped characters requires allocating a new
                    // `Bytes` buffer. We'd rather not re-allocate and instead just
                    // point to the buffer from which everything was parsed.
                    if value.contains(&b'\\') {
                        value = Bytes::from_iter(EscapeBytes::new(value.into_iter()))
                    }
                    match op.as_bytes() {
                        OPERATOR_BYTES_ASSIGN => AstEntry::new_assign(key, value),
                        OPERATOR_BYTES_ASSIGN_IF_UNDEFINED => {
                            AstEntry::new_assign_if_undefined(key, value)
                        }
                        OPERATOR_BYTES_ADD => AstEntry::new_add(key, value),
                        OPERATOR_BYTES_REMOVE => AstEntry::new_remove(key, value),
                        _ => panic!(
                            "operator should not match assign operators: {}",
                            OsStr::from_bytes(op.as_bytes()).display()
                        ),
                    }
                } else {
                    let op = captured
                        .name(CAPTURE_KVP_RESET_OPERATOR)
                        .expect("an operation must be present if key-op-value key is found");
                    match op.as_bytes() {
                        OPERATOR_BYTES_RESET => AstEntry::new_reset(key),
                        _ => panic!(
                            "operator should not match reset operators: {}",
                            OsStr::from_bytes(op.as_bytes()).display()
                        ),
                    }
                };
                stack
                    .last_mut()
                    .expect("stack initialized with one element")
                    .entries
                    .push(entry);
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
            Ok(AstTree::from_iter(tree.entries))
        }
    }
}

struct EscapeBytes<I>(I);
impl<I> EscapeBytes<I>
where
    I: Iterator<Item = u8>,
{
    fn new(unescaped_string: I) -> Self {
        Self(unescaped_string)
    }
}
impl<I: Iterator> Iterator for EscapeBytes<I>
where
    I: Iterator<Item = u8>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let byte = self.0.next()?;
        if byte == b'\\' {
            Some(self.0.next().unwrap_or(byte))
        } else {
            Some(byte)
        }
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::ast::{AstEntry, AstParser, AstTree};

    #[rstest]
    #[case(b"")]
    #[case(b" ")]
    #[case(b"\t")]
    #[case(b"\n")]
    #[case(b"\r\n")]
    #[case(b" \t\n\r\n")]
    fn parse_empty_ast(#[case] input: &[u8]) {
        let ast = AstParser::new()
            .parse_bytes(input.to_vec())
            .parse_into_tree();
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), AstTree::new());
    }

    #[rstest]
    #[case(
        b"KEY=UNQUOTED_STRING",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
    #[case(
        b"KEY_WITH_UNDERSCORES=UNQUOTED_STRING/0123456789.",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY_WITH_UNDERSCORES".to_vec(), b"UNQUOTED_STRING/0123456789.".to_vec())
        ])
    )]
    #[case(
        b"KEY=\"QUOTED String 0123456789 \\\\ \\\"\"",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String 0123456789 \\ \"".to_vec())
        ])
    )]
    #[case(
        b"KEY=UNQUOTED_STRING;",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
    #[case(
        b"KEY = UNQUOTED_STRING ;",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
    #[case(
        b"KEY\n=\n\t    UNQUOTED_STRING;",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
    #[case(
        b"KEY=UNQUOTED_STRING;KEY2=\"QUOTED String @\";",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY2".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
    #[case(
        b"
        KEY+=UNQUOTED_STRING
        KEY=\"QUOTED String @\";
        ",
        AstTree::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
    #[case(
        b"KEY+=UNQUOTED_STRING",
        AstTree::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
    #[case(
        b"KEY-=UNQUOTED_STRING",
        AstTree::from_iter(vec![
            AstEntry::new_remove(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
    #[case(
        b"KEY:=UNQUOTED_STRING",
        AstTree::from_iter(vec![
            AstEntry::new_assign_if_undefined(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
    #[case(
        b"KEY!",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec())
        ])
    )]
    #[case(
        b"KEY !",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec())
        ])
    )]
    #[case(
        b"KEY !",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec())
        ])
    )]
    #[case(
        b"KEY!
        KEY=\"QUOTED String @\";
        ",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
    #[case(
        b"KEY!NEXT-=VALUE",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec()),
            AstEntry::new_remove(b"NEXT".to_vec(), b"VALUE".to_vec()),
        ])
    )]
    fn parse_key_op_value(#[case] input: &[u8], #[case] output: AstTree) {
        use crate::ast::parser::AstParser;

        let ast = AstParser::new()
            .parse_bytes(input.to_vec())
            .parse_into_tree();
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), output);
    }

    #[rstest]
    #[case(
        b"KEY: {}",
        AstTree::from_iter(vec![
            AstEntry::new_group(b"KEY".to_vec(), vec![])
        ])
    )]
    #[case(
        b"      \t_   \n: {\t     }",
        AstTree::from_iter(vec![
            AstEntry::new_group(b"_".to_vec(), vec![])
        ])
    )]
    #[case(
        b"KEY: {
            PART1 = VALUE;
            PART2 = other
        }",
        AstTree::from_iter(vec![
            AstEntry::new_group(
                b"KEY".to_vec(),
                vec![
                    AstEntry::new_assign(b"PART1".to_vec(), b"VALUE".to_vec()),
                    AstEntry::new_assign(b"PART2".to_vec(), b"other".to_vec()),
                ]
            )
        ])
    )]
    fn parse_group(#[case] input: &[u8], #[case] output: AstTree) {
        let ast = AstParser::new()
            .parse_bytes(input.to_vec())
            .parse_into_tree();
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), output);
    }
}

#[cfg(test)]
mod property_test {
    use std::{ffi::OsStr, ops::Deref, os::unix::ffi::OsStrExt};

    use bytes::Bytes;
    use proptest::prelude::*;
    use proptest_derive::Arbitrary;

    use crate::ast::{
        AstEntry, AstTree,
        parser::{
            AstParser, EscapeBytes, OPERATOR_BYTES_ADD, OPERATOR_BYTES_ASSIGN,
            OPERATOR_BYTES_ASSIGN_IF_UNDEFINED, OPERATOR_BYTES_REMOVE, OPERATOR_BYTES_RESET,
        },
    };

    trait AsBytes {
        fn bytes(&self) -> impl Iterator<Item = u8>;
    }
    trait AsAstEntry: AsBytes {
        fn as_input(&self) -> Bytes {
            Vec::from_iter(self.bytes()).into()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropWhitespace {
        #[proptest(regex = r"(?s-u:(\s|\r\n|\n)+)")]
        val: Vec<u8>,
    }
    impl AsBytes for PropWhitespace {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.val.iter().copied()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropOptionalWhitespace {
        val: Option<PropWhitespace>,
    }
    impl AsBytes for PropOptionalWhitespace {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.val
                .as_ref()
                .map(|whitespace| whitespace.val.as_slice())
                .unwrap_or_default()
                .iter()
                .copied()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropTerminator {
        #[proptest(regex = r";")]
        val: Vec<u8>,
    }
    impl AsBytes for PropTerminator {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.val.iter().copied()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropOptionalTerminator {
        val: Option<PropTerminator>,
    }
    impl AsBytes for PropOptionalTerminator {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.val
                .as_ref()
                .map(|whitespace| whitespace.val.as_slice())
                .unwrap_or_default()
                .iter()
                .copied()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropIdentifier {
        #[proptest(regex = r"[A-Za-z0-9_.]+|[A-Za-z0-9_.][A-Za-z0-9_./\-:]*[A-Za-z0-9_.]")]
        val: Vec<u8>,
    }
    impl AsBytes for PropIdentifier {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.val.iter().copied()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropAssignOp {
        #[proptest(regex = r"=|:=|\+=|-=")]
        val: Vec<u8>,
    }
    impl AsBytes for PropAssignOp {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.val.iter().copied()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropResetOp {
        #[proptest(regex = r"!")]
        val: Vec<u8>,
    }
    impl AsBytes for PropResetOp {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.val.iter().copied()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropUnquotedString {
        #[proptest(regex = r"(?s-u:[A-Za-z0-9_./\-]+)")]
        val: Vec<u8>,
    }
    impl AsBytes for PropUnquotedString {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.val.iter().copied()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropQuotedString {
        #[proptest(regex = r#"(?s-u:([^\\"]|\\\.)*)"#)]
        val: Vec<u8>,
    }
    impl AsBytes for PropQuotedString {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.val.iter().copied()
        }
    }

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropKeyAssignUnquotedValue {
        sep0: PropOptionalWhitespace,
        identifier: PropIdentifier,
        sep1: PropOptionalWhitespace,
        op: PropAssignOp,
        sep2: PropOptionalWhitespace,
        value: PropUnquotedString,
        sep3: PropOptionalWhitespace,
        terminator: PropOptionalTerminator,
    }
    impl PropKeyAssignUnquotedValue {
        pub fn as_ast_entry(&self) -> AstEntry {
            match self.op.val.deref() {
                OPERATOR_BYTES_ASSIGN => {
                    AstEntry::new_assign(self.identifier.val.clone(), self.value.val.clone())
                }
                OPERATOR_BYTES_ASSIGN_IF_UNDEFINED => AstEntry::new_assign_if_undefined(
                    self.identifier.val.clone(),
                    self.value.val.clone(),
                ),
                OPERATOR_BYTES_ADD => {
                    AstEntry::new_add(self.identifier.val.clone(), self.value.val.clone())
                }
                OPERATOR_BYTES_REMOVE => {
                    AstEntry::new_remove(self.identifier.val.clone(), self.value.val.clone())
                }
                OPERATOR_BYTES_RESET => AstEntry::new_reset(self.identifier.val.clone()),
                _ => panic!(
                    "operator not supported: {}",
                    OsStr::from_bytes(self.op.val.deref()).display()
                ),
            }
        }
    }
    impl AsBytes for PropKeyAssignUnquotedValue {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.sep0
                .bytes()
                .chain(self.identifier.bytes())
                .chain(self.sep1.bytes())
                .chain(self.op.bytes())
                .chain(self.sep2.bytes())
                .chain(self.value.bytes())
                .chain(self.sep3.bytes())
                .chain(self.terminator.bytes())
        }
    }
    impl AsAstEntry for PropKeyAssignUnquotedValue {}

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropKeyAssignQuotedValue {
        sep0: PropOptionalWhitespace,
        identifier: PropIdentifier,
        sep1: PropOptionalWhitespace,
        op: PropAssignOp,
        sep2: PropOptionalWhitespace,
        value: PropQuotedString,
        sep3: PropOptionalWhitespace,
        terminator: PropOptionalTerminator,
    }
    impl PropKeyAssignQuotedValue {
        pub fn as_ast_entry(&self) -> AstEntry {
            let value = EscapeBytes::new(self.value.val.iter().copied());
            match self.op.val.deref() {
                OPERATOR_BYTES_ASSIGN => {
                    AstEntry::new_assign(self.identifier.val.clone(), value.collect::<Bytes>())
                }
                OPERATOR_BYTES_ASSIGN_IF_UNDEFINED => AstEntry::new_assign_if_undefined(
                    self.identifier.val.clone(),
                    value.collect::<Bytes>(),
                ),
                OPERATOR_BYTES_ADD => {
                    AstEntry::new_add(self.identifier.val.clone(), value.collect::<Bytes>())
                }
                OPERATOR_BYTES_REMOVE => {
                    AstEntry::new_remove(self.identifier.val.clone(), value.collect::<Bytes>())
                }
                OPERATOR_BYTES_RESET => AstEntry::new_reset(self.identifier.val.clone()),
                _ => panic!(
                    "operator not supported: {}",
                    OsStr::from_bytes(self.op.val.deref()).display()
                ),
            }
        }
    }
    impl AsBytes for PropKeyAssignQuotedValue {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.sep0
                .bytes()
                .chain(self.identifier.bytes())
                .chain(self.sep1.bytes())
                .chain(self.op.bytes())
                .chain(self.sep2.bytes())
                .chain(b"\"".iter().copied())
                .chain(self.value.bytes())
                .chain(b"\"".iter().copied())
                .chain(self.sep3.bytes())
                .chain(self.terminator.bytes())
        }
    }
    impl AsAstEntry for PropKeyAssignQuotedValue {}

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropKeyReset {
        sep0: PropOptionalWhitespace,
        identifier: PropIdentifier,
        sep1: PropOptionalWhitespace,
        op: PropResetOp,
        sep2: PropOptionalWhitespace,
        terminator: PropOptionalTerminator,
    }
    impl PropKeyReset {
        pub fn as_ast_entry(&self) -> AstEntry {
            AstEntry::new_reset(self.identifier.val.clone())
        }
    }
    impl AsBytes for PropKeyReset {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.sep0
                .bytes()
                .chain(self.identifier.bytes())
                .chain(self.sep1.bytes())
                .chain(self.op.bytes())
                .chain(self.sep2.bytes())
                .chain(self.terminator.bytes())
        }
    }
    impl AsAstEntry for PropKeyReset {}

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    enum PropKeyOpValue {
        Unquoted(PropKeyAssignUnquotedValue),
        Quoted(PropKeyAssignQuotedValue),
        Reset(PropKeyReset),
    }
    impl PropKeyOpValue {
        pub fn as_ast_entry(&self) -> AstEntry {
            match self {
                Self::Unquoted(kov) => kov.as_ast_entry(),
                Self::Quoted(kov) => kov.as_ast_entry(),
                Self::Reset(kov) => kov.as_ast_entry(),
            }
        }
    }
    impl AsBytes for PropKeyOpValue {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            let bytes: Box<dyn Iterator<Item = u8>> = match self {
                Self::Unquoted(kov) => Box::new(kov.bytes()),
                Self::Quoted(kov) => Box::new(kov.bytes()),
                Self::Reset(kov) => Box::new(kov.bytes()),
            };
            bytes
        }
    }
    impl AsAstEntry for PropKeyOpValue {}

    #[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
    struct PropGroup {
        sep0: PropOptionalWhitespace,
        identifier: PropIdentifier,
        sep1: PropOptionalWhitespace,
        #[proptest(regex = r":")]
        op: Vec<u8>,
        sep2: PropOptionalWhitespace,
        #[proptest(regex = r"\{")]
        body_start: Vec<u8>,
        sep3: PropOptionalWhitespace,
        entries: Vec<(PropKeyOpValue, PropWhitespace)>,
        sep4: PropOptionalWhitespace,
        #[proptest(regex = r"\}")]
        body_end: Vec<u8>,
        sep5: PropOptionalWhitespace,
        terminator: PropOptionalTerminator,
    }
    impl PropGroup {
        pub fn as_ast_entry(&self) -> AstEntry {
            AstEntry::new_group(
                self.identifier.val.clone(),
                self.entries.iter().map(|(e, _)| e.as_ast_entry()),
            )
        }
    }
    impl AsBytes for PropGroup {
        fn bytes(&self) -> impl Iterator<Item = u8> {
            self.sep0
                .bytes()
                .chain(self.identifier.bytes())
                .chain(self.sep1.bytes())
                .chain(self.op.iter().copied())
                .chain(self.sep2.bytes())
                .chain(self.body_start.iter().copied())
                .chain(self.sep3.bytes())
                .chain(
                    self.entries
                        .iter()
                        .flat_map(|(e, w)| e.bytes().chain(w.bytes())),
                )
                .chain(self.sep4.bytes())
                .chain(self.body_end.iter().copied())
                .chain(self.sep5.bytes())
                .chain(self.terminator.bytes())
        }
    }
    impl AsAstEntry for PropGroup {}

    proptest! {
        #[test]
        fn no_panics(input in ".*") {
            let _ = AstParser::new().parse_bytes(input).parse_into_tree();
        }

        #[test]
        fn single_key_op_unquoted_value(kov: PropKeyAssignUnquotedValue) {
            let expected_ast = AstTree {
                entries: vec![kov.as_ast_entry()]
            };
            let ast = AstParser::new().parse_bytes(kov.as_input()).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_op_unquoted_value_and_key_op_unquoted_value(
            kov1: PropKeyAssignUnquotedValue,
            sep: PropWhitespace,
            kov2: PropKeyAssignUnquotedValue,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(sep.bytes()).chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_op_unquoted_value_and_key_op_quoted_value(
            kov1: PropKeyAssignUnquotedValue,
            sep: PropWhitespace,
            kov2: PropKeyAssignQuotedValue,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(sep.bytes()).chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_op_unquoted_value_and_key_reset(
            kov1: PropKeyAssignUnquotedValue,
            sep: PropWhitespace,
            kov2: PropKeyReset,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(sep.bytes()).chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_op_unquoted_value_and_group(
            kov1: PropKeyAssignUnquotedValue,
            sep: PropWhitespace,
            kov2: PropGroup,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(sep.bytes()).chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn single_key_op_quoted_value(kov: PropKeyAssignQuotedValue) {
            let expected_ast = AstTree {
                entries: vec![kov.as_ast_entry()]
            };
            let ast = AstParser::new().parse_bytes(kov.as_input()).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_op_quoted_value_and_key_op_unquoted_value(
            kov1: PropKeyAssignQuotedValue,
            kov2: PropKeyAssignUnquotedValue,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_op_quoted_value_and_key_op_quoted_value(
            kov1: PropKeyAssignQuotedValue,
            kov2: PropKeyAssignQuotedValue,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_op_quoted_value_and_key_reset(
            kov1: PropKeyAssignQuotedValue,
            kov2: PropKeyReset,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_op_quoted_value_and_group(
            kov1: PropKeyAssignQuotedValue,
            kov2: PropGroup,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn single_key_reset(kov: PropKeyReset) {
            let expected_ast = AstTree {
                entries: vec![kov.as_ast_entry()]
            };
            let ast = AstParser::new().parse_bytes(kov.as_input()).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_reset_and_key_op_unquoted_value(
            kov1: PropKeyReset,
            kov2: PropKeyAssignUnquotedValue,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_reset_and_key_op_quoted_value(
            kov1: PropKeyReset,
            kov2: PropKeyAssignQuotedValue,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_reset_and_key_reset(
            kov1: PropKeyReset,
            kov2: PropKeyReset,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn key_reset_and_group(
            kov1: PropKeyReset,
            kov2: PropGroup,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn single_group(kov: PropGroup) {
            let expected_ast = AstTree {
                entries: vec![kov.as_ast_entry()]
            };
            let ast = AstParser::new().parse_bytes(kov.as_input()).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }

        #[test]
        fn group_and_group(
            kov1: PropGroup,
            kov2: PropGroup,
        ) {
            let expected_ast = AstTree {
                entries: vec![
                    kov1.as_ast_entry(),
                    kov2.as_ast_entry(),
                ]
            };
            let ast = AstParser::new().parse_bytes(
                Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
            ).parse_into_tree();
            assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
            assert_eq!(ast.unwrap(), expected_ast);
        }
    }
}
