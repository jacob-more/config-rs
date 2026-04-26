use std::{ffi::OsStr, ops::Deref, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use proptest::prelude::*;
use proptest_derive::Arbitrary;

use crate::{
    ext::IterEscaped,
    parse::{
        Ast, AstEntry, AstParser, BYTES_OPERATOR_ADD, BYTES_OPERATOR_ASSIGN,
        BYTES_OPERATOR_ASSIGN_IF_UNDEFINED, BYTES_OPERATOR_CLEAR, BYTES_OPERATOR_REMOVE,
        BYTES_OPERATOR_RESET,
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
enum PropString {
    Quoted(#[proptest(regex = r#"(?s-u:[^"\\]|\\.)*"#)] Vec<u8>),
    Unquoted(
        #[proptest(regex = r#"(?s-u:[A-Za-z0-9_./](?:[A-Za-z0-9_./\-:]*[A-Za-z0-9_./])?)"#)]
        Vec<u8>,
    ),
}
impl PropString {
    fn as_ast_string(&self) -> Bytes {
        match self {
            Self::Quoted(items) => items,
            Self::Unquoted(items) => items,
        }
        .iter()
        .unescaped()
        .copied()
        .collect()
    }
}
impl AsBytes for PropString {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        match self {
            Self::Quoted(items) => b"\"".iter().chain(items.iter()).chain(b"\"".iter()),
            // Use empty strings so that the return types match. I prefer this
            // to using Box<dyn> despite not having benchmarked the performance
            // difference.
            Self::Unquoted(items) => b"".iter().chain(items.iter()).chain(b"".iter()),
        }
        .copied()
    }
}

#[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropKeyAssignValue {
    sep0: PropOptionalWhitespace,
    identifier: PropString,
    sep1: PropOptionalWhitespace,
    op: PropAssignOp,
    sep2: PropOptionalWhitespace,
    value: PropString,
    sep3: PropOptionalWhitespace,
    terminator: PropOptionalTerminator,
}
impl PropKeyAssignValue {
    pub fn as_ast_entry(&self) -> AstEntry {
        match self.op.val.deref() {
            BYTES_OPERATOR_ASSIGN => {
                AstEntry::new_assign(self.identifier.as_ast_string(), self.value.as_ast_string())
            }
            BYTES_OPERATOR_ASSIGN_IF_UNDEFINED => AstEntry::new_assign_if_undefined(
                self.identifier.as_ast_string(),
                self.value.as_ast_string(),
            ),
            BYTES_OPERATOR_ADD => {
                AstEntry::new_add(self.identifier.as_ast_string(), self.value.as_ast_string())
            }
            BYTES_OPERATOR_REMOVE => {
                AstEntry::new_remove(self.identifier.as_ast_string(), self.value.as_ast_string())
            }
            BYTES_OPERATOR_RESET => AstEntry::new_reset(self.identifier.as_ast_string()),
            _ => panic!(
                "operator not supported: {}",
                OsStr::from_bytes(self.op.val.deref()).display()
            ),
        }
    }
}
impl AsBytes for PropKeyAssignValue {
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
impl AsAstEntry for PropKeyAssignValue {}

#[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropKeyReset {
    sep0: PropOptionalWhitespace,
    identifier: PropString,
    sep1: PropOptionalWhitespace,
    op: PropResetOp,
    sep2: PropOptionalWhitespace,
    terminator: PropOptionalTerminator,
}
impl PropKeyReset {
    pub fn as_ast_entry(&self) -> AstEntry {
        match self.op.val.deref() {
            BYTES_OPERATOR_CLEAR => AstEntry::new_clear(self.identifier.as_ast_string()),
            BYTES_OPERATOR_RESET => AstEntry::new_reset(self.identifier.as_ast_string()),
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
    Assign(PropKeyAssignValue),
    Reset(PropKeyReset),
}
impl PropKeyOpValue {
    pub fn as_ast_entry(&self) -> AstEntry {
        match self {
            Self::Assign(kov) => kov.as_ast_entry(),
            Self::Reset(kov) => kov.as_ast_entry(),
        }
    }
}
impl AsBytes for PropKeyOpValue {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        let bytes: Box<dyn Iterator<Item = u8>> = match self {
            Self::Assign(kov) => Box::new(kov.bytes()),
            Self::Reset(kov) => Box::new(kov.bytes()),
        };
        bytes
    }
}
impl AsAstEntry for PropKeyOpValue {}

#[derive(Debug, Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropGroup {
    sep0: PropOptionalWhitespace,
    identifier: PropString,
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
            self.identifier.as_ast_string(),
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
        let _ = AstParser::new().parse(input);
    }

    #[test]
    fn single_key_op_unquoted_value(kov: PropKeyAssignValue) {
        let expected_ast = Ast {
            entries: vec![kov.as_ast_entry()]
        };
        let ast = AstParser::new().parse(kov.as_input());
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn key_op_value_and_key_op_value(
        kov1: PropKeyAssignValue,
        sep: PropWhitespace,
        kov2: PropKeyAssignValue,
    ) {
        let expected_ast = Ast {
            entries: vec![
                kov1.as_ast_entry(),
                kov2.as_ast_entry(),
            ]
        };
        let ast = AstParser::new().parse(
            Vec::from_iter(kov1.bytes().chain(sep.bytes()).chain(kov2.bytes()))
        );
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn key_op_value_and_key_reset(
        kov1: PropKeyAssignValue,
        sep: PropWhitespace,
        kov2: PropKeyReset,
    ) {
        let expected_ast = Ast {
            entries: vec![
                kov1.as_ast_entry(),
                kov2.as_ast_entry(),
            ]
        };
        let ast = AstParser::new().parse(
            Vec::from_iter(kov1.bytes().chain(sep.bytes()).chain(kov2.bytes()))
        );
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn key_op_value_and_group(
        kov1: PropKeyAssignValue,
        sep: PropWhitespace,
        kov2: PropGroup,
    ) {
        let expected_ast = Ast {
            entries: vec![
                kov1.as_ast_entry(),
                kov2.as_ast_entry(),
            ]
        };
        let ast = AstParser::new().parse(
            Vec::from_iter(kov1.bytes().chain(sep.bytes()).chain(kov2.bytes()))
        );
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn single_key_op_value(kov: PropKeyAssignValue) {
        let expected_ast = Ast {
            entries: vec![kov.as_ast_entry()]
        };
        let ast = AstParser::new().parse(kov.as_input());
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn single_key_reset(kov: PropKeyReset) {
        let expected_ast = Ast {
            entries: vec![kov.as_ast_entry()]
        };
        let ast = AstParser::new().parse(kov.as_input());
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn key_reset_and_key_op_value(
        kov1: PropKeyReset,
        kov2: PropKeyAssignValue,
    ) {
        let expected_ast = Ast {
            entries: vec![
                kov1.as_ast_entry(),
                kov2.as_ast_entry(),
            ]
        };
        let ast = AstParser::new().parse(
            Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
        );
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn key_reset_and_key_reset(
        kov1: PropKeyReset,
        kov2: PropKeyReset,
    ) {
        let expected_ast = Ast {
            entries: vec![
                kov1.as_ast_entry(),
                kov2.as_ast_entry(),
            ]
        };
        let ast = AstParser::new().parse(
            Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
        );
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn key_reset_and_group(
        kov1: PropKeyReset,
        kov2: PropGroup,
    ) {
        let expected_ast = Ast {
            entries: vec![
                kov1.as_ast_entry(),
                kov2.as_ast_entry(),
            ]
        };
        let ast = AstParser::new().parse(
            Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
        );
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn single_group(kov: PropGroup) {
        let expected_ast = Ast {
            entries: vec![kov.as_ast_entry()]
        };
        let ast = AstParser::new().parse(kov.as_input());
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }

    #[test]
    fn group_and_group(
        kov1: PropGroup,
        kov2: PropGroup,
    ) {
        let expected_ast = Ast {
            entries: vec![
                kov1.as_ast_entry(),
                kov2.as_ast_entry(),
            ]
        };
        let ast = AstParser::new().parse(
            Vec::from_iter(kov1.bytes().chain(kov2.bytes()))
        );
        assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
        assert_eq!(ast.unwrap(), expected_ast);
    }
}
