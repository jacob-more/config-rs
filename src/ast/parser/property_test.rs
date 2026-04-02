use std::{ffi::OsStr, ops::Deref, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use proptest::prelude::*;
use proptest_derive::Arbitrary;

use crate::ast::{
    AstEntry, AstTree,
    parser::{
        AstParser, EscapeBytes, OPERATOR_BYTES_ADD, OPERATOR_BYTES_ASSIGN,
        OPERATOR_BYTES_ASSIGN_IF_UNDEFINED, OPERATOR_BYTES_CLEAR, OPERATOR_BYTES_REMOVE,
        OPERATOR_BYTES_RESET,
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
    #[proptest(regex = r"[A-Za-z0-9_./]+|[A-Za-z0-9_./][A-Za-z0-9_./\-:]*[A-Za-z0-9_./]")]
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
    #[proptest(regex = r"!|!!")]
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
            _ => panic!(
                "assign operator not supported: {}",
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
        match self.op.val.deref() {
            OPERATOR_BYTES_CLEAR => AstEntry::new_clear(self.identifier.val.clone()),
            OPERATOR_BYTES_RESET => AstEntry::new_reset(self.identifier.val.clone()),
            _ => panic!(
                "reset operator not supported: {}",
                OsStr::from_bytes(self.op.val.deref()).display()
            ),
        }
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
