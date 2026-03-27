use std::{ffi::OsStr, fmt::Display, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use config_ast::{
    AstEntry, AstTree, OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_ASSIGN_IF_UNDEFINED,
    OPERATOR_REMOVE, OPERATOR_RESET,
};
use display_ext::Join;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReprIrParseError {
    #[error(
        "operator '{}' is not known\n{} {} {}",
        OsStr::from_bytes(operator).display(),
        OsStr::from_bytes(name).display(),
        OsStr::from_bytes(operator).display(),
        OsStr::from_bytes(value).display()
    )]
    UknownOperator {
        name: Bytes,
        operator: Bytes,
        value: Bytes,
    },
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct IrParseError(#[from] ReprIrParseError);

const OPERATOR_BYTES_ASSIGN: &[u8] = OPERATOR_ASSIGN.as_bytes();
const OPERATOR_BYTES_ASSIGN_IF_UNDEFINED: &[u8] = OPERATOR_ASSIGN_IF_UNDEFINED.as_bytes();
const OPERATOR_BYTES_ADD: &[u8] = OPERATOR_ADD.as_bytes();
const OPERATOR_BYTES_REMOVE: &[u8] = OPERATOR_REMOVE.as_bytes();
const OPERATOR_BYTES_RESET: &[u8] = OPERATOR_RESET.as_bytes();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrTree {
    entries: Vec<IrEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrEntry {
    Group { name: Bytes, entries: Vec<IrEntry> },
    Assign { name: Bytes, value: Bytes },
    AssignIfUndefined { name: Bytes, value: Bytes },
    Add { name: Bytes, value: Bytes },
    Remove { name: Bytes, value: Bytes },
    Reset { name: Bytes },
}

impl IrTree {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn from_ast(ast: AstTree) -> Result<Self, IrParseError> {
        let mut ir_entries = Vec::new();
        for entry in ast.into_entries() {
            ir_entries.push(IrEntry::from_ast(entry)?);
        }
        Ok(Self::from_iter(ir_entries))
    }

    pub fn entries(&self) -> impl Iterator<Item = &'_ IrEntry> {
        self.entries.iter()
    }

    pub fn into_entries(self) -> impl Iterator<Item = IrEntry> {
        self.entries.into_iter()
    }
}

impl Default for IrTree {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<IrEntry> for IrTree {
    fn from_iter<T: IntoIterator<Item = IrEntry>>(entries: T) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

impl IrEntry {
    pub fn from_ast(ast: AstEntry) -> Result<Self, IrParseError> {
        match ast {
            AstEntry::Group { name, entries } => {
                let mut ir_entries = Vec::with_capacity(entries.len());
                for entry in entries {
                    ir_entries.push(Self::from_ast(entry)?);
                }
                Ok(IrEntry::new_group(name, ir_entries))
            }
            AstEntry::KeyOpValue {
                key,
                operator,
                mut value,
            } => {
                // Replacing escaped characters requires allocating a new
                // `Bytes` buffer. We'd rather not re-allocate and instead just
                // point to the buffer from which everything was parsed.
                if value.contains(&b'\\') {
                    value = Bytes::from_iter(EscapeBytes::new(value.into_iter()))
                }
                match &*operator {
                    OPERATOR_BYTES_ASSIGN => Ok(Self::new_assign(key, value)),
                    OPERATOR_BYTES_ASSIGN_IF_UNDEFINED => {
                        Ok(Self::new_assign_if_undefined(key, value))
                    }
                    OPERATOR_BYTES_ADD => Ok(Self::new_add(key, value)),
                    OPERATOR_BYTES_REMOVE => Ok(Self::new_remove(key, value)),
                    OPERATOR_BYTES_RESET => Ok(Self::new_reset(key)),
                    _ => Err(IrParseError(ReprIrParseError::UknownOperator {
                        name: key,
                        operator,
                        value,
                    })),
                }
            }
        }
    }

    pub fn new_group(name: impl Into<Bytes>, values: impl IntoIterator<Item = IrEntry>) -> Self {
        Self::Group {
            name: name.into(),
            entries: values.into_iter().collect(),
        }
    }

    pub fn new_assign(name: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Assign {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn new_assign_if_undefined(name: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::AssignIfUndefined {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn new_add(name: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Add {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn new_remove(name: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Remove {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn new_reset(name: impl Into<Bytes>) -> Self {
        Self::Reset { name: name.into() }
    }
}

impl Display for IrTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Join::new(self.entries.iter(), ' '))
    }
}

impl Display for IrEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn display_key_op_value(
            f: &mut std::fmt::Formatter<'_>,
            key: &Bytes,
            op: &str,
            value: &Bytes,
        ) -> std::fmt::Result {
            write!(
                f,
                "{} {op} \"{:?}\";",
                OsStr::from_bytes(key).display(),
                OsStr::from_bytes(value).display()
            )
        }

        match self {
            Self::Group { name, entries } => {
                write!(f, "{}: {{", OsStr::from_bytes(name).display())?;
                if entries.is_empty() {
                    write!(f, "{} }}", Join::new(entries.iter(), ", "))?;
                } else {
                    write!(f, "}};")?;
                }
                Ok(())
            }
            Self::Assign { name, value } => display_key_op_value(f, name, OPERATOR_ASSIGN, value),
            Self::AssignIfUndefined { name, value } => {
                display_key_op_value(f, name, OPERATOR_ASSIGN_IF_UNDEFINED, value)
            }
            Self::Add { name, value } => display_key_op_value(f, name, OPERATOR_ADD, value),
            Self::Remove { name, value } => display_key_op_value(f, name, OPERATOR_REMOVE, value),
            Self::Reset { name } => {
                write!(f, "{} {OPERATOR_RESET};", OsStr::from_bytes(name).display())
            }
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
    use config_ast::AstEntry;
    use rstest::rstest;

    use crate::IrEntry;

    #[rstest]
    #[case(AstEntry::new_group(br"KEY".to_vec(), vec![]), IrEntry::new_group(br"KEY".to_vec(), vec![]))]
    #[case(AstEntry::new_group(br"KEY".to_vec(), vec![
        AstEntry::new_assign(br"CHILD".to_vec(), br"VALUE".to_vec())
    ]), IrEntry::new_group(br"KEY".to_vec(), vec![
        IrEntry::new_assign(br"CHILD".to_vec(), br"VALUE".to_vec())
    ]))]
    #[case(AstEntry::new_group(br"KEY".to_vec(), vec![
        AstEntry::new_assign(br"CHILD".to_vec(), br"VALUE".to_vec()),
        AstEntry::new_add(br"CHILD".to_vec(), br"VALUE\\trailing text".to_vec()),
    ]), IrEntry::new_group(br"KEY".to_vec(), vec![
        IrEntry::new_assign(br"CHILD".to_vec(), br"VALUE".to_vec()),
        IrEntry::new_add(br"CHILD".to_vec(), br"VALUE\trailing text".to_vec()),
    ]))]
    fn parse_group_ir_entry(#[case] ast: AstEntry, #[case] expected_ir: IrEntry) {
        let ir = IrEntry::from_ast(ast.clone());
        assert!(ir.is_ok(), "error converting AST to IR: {ast:?}");
        assert_eq!(ir.unwrap(), expected_ir);
    }

    #[rstest]
    #[case(AstEntry::new_assign(br"KEY".to_vec(), br"VALUE".to_vec()), IrEntry::new_assign(br"KEY".to_vec(), br"VALUE".to_vec()))]
    #[case(AstEntry::new_assign(br"KEY".to_vec(), br"VALUE\".to_vec()), IrEntry::new_assign(br"KEY".to_vec(), br"VALUE\".to_vec()))]
    #[case(AstEntry::new_assign(br"KEY".to_vec(), br"VALUE\\".to_vec()), IrEntry::new_assign(br"KEY".to_vec(), br"VALUE\".to_vec()))]
    #[case(AstEntry::new_assign(br"KEY".to_vec(), br"VALUE\\trailing text".to_vec()), IrEntry::new_assign(br"KEY".to_vec(), br"VALUE\trailing text".to_vec()))]
    #[case(AstEntry::new_assign(br"KEY".to_vec(), br#"\"trailing text"#.to_vec()), IrEntry::new_assign(br"KEY".to_vec(), br#""trailing text"#.to_vec()))]
    fn parse_assign_ir_entry(#[case] ast: AstEntry, #[case] expected_ir: IrEntry) {
        let ir = IrEntry::from_ast(ast.clone());
        assert!(ir.is_ok(), "error converting AST to IR: {ast:?}");
        assert_eq!(ir.unwrap(), expected_ir);
    }

    #[rstest]
    #[case(AstEntry::new_assign_if_undefined(br"KEY".to_vec(), br"VALUE".to_vec()), IrEntry::new_assign_if_undefined(br"KEY".to_vec(), br"VALUE".to_vec()))]
    #[case(AstEntry::new_assign_if_undefined(br"KEY".to_vec(), br"VALUE\".to_vec()), IrEntry::new_assign_if_undefined(br"KEY".to_vec(), br"VALUE\".to_vec()))]
    #[case(AstEntry::new_assign_if_undefined(br"KEY".to_vec(), br"VALUE\\".to_vec()), IrEntry::new_assign_if_undefined(br"KEY".to_vec(), br"VALUE\".to_vec()))]
    #[case(AstEntry::new_assign_if_undefined(br"KEY".to_vec(), br"VALUE\\trailing text".to_vec()), IrEntry::new_assign_if_undefined(br"KEY".to_vec(), br"VALUE\trailing text".to_vec()))]
    #[case(AstEntry::new_assign_if_undefined(br"KEY".to_vec(), br#"\"trailing text"#.to_vec()), IrEntry::new_assign_if_undefined(br"KEY".to_vec(), br#""trailing text"#.to_vec()))]
    fn parse_assign_if_undefined_ir_entry(#[case] ast: AstEntry, #[case] expected_ir: IrEntry) {
        let ir = IrEntry::from_ast(ast.clone());
        assert!(ir.is_ok(), "error converting AST to IR: {ast:?}");
        assert_eq!(ir.unwrap(), expected_ir);
    }

    #[rstest]
    #[case(AstEntry::new_add(br"KEY".to_vec(), br"VALUE".to_vec()), IrEntry::new_add(br"KEY".to_vec(), br"VALUE".to_vec()))]
    #[case(AstEntry::new_add(br"KEY".to_vec(), br"VALUE\".to_vec()), IrEntry::new_add(br"KEY".to_vec(), br"VALUE\".to_vec()))]
    #[case(AstEntry::new_add(br"KEY".to_vec(), br"VALUE\\".to_vec()), IrEntry::new_add(br"KEY".to_vec(), br"VALUE\".to_vec()))]
    #[case(AstEntry::new_add(br"KEY".to_vec(), br"VALUE\\trailing text".to_vec()), IrEntry::new_add(br"KEY".to_vec(), br"VALUE\trailing text".to_vec()))]
    #[case(AstEntry::new_add(br"KEY".to_vec(), br#"\"trailing text"#.to_vec()), IrEntry::new_add(br"KEY".to_vec(), br#""trailing text"#.to_vec()))]
    fn parse_add_ir_entry(#[case] ast: AstEntry, #[case] expected_ir: IrEntry) {
        let ir = IrEntry::from_ast(ast.clone());
        assert!(ir.is_ok(), "error converting AST to IR: {ast:?}");
        assert_eq!(ir.unwrap(), expected_ir);
    }

    #[rstest]
    #[case(AstEntry::new_remove(br"KEY".to_vec(), br"VALUE".to_vec()), IrEntry::new_remove(br"KEY".to_vec(), br"VALUE".to_vec()))]
    #[case(AstEntry::new_remove(br"KEY".to_vec(), br"VALUE\".to_vec()), IrEntry::new_remove(br"KEY".to_vec(), br"VALUE\".to_vec()))]
    #[case(AstEntry::new_remove(br"KEY".to_vec(), br"VALUE\\".to_vec()), IrEntry::new_remove(br"KEY".to_vec(), br"VALUE\".to_vec()))]
    #[case(AstEntry::new_remove(br"KEY".to_vec(), br"VALUE\\trailing text".to_vec()), IrEntry::new_remove(br"KEY".to_vec(), br"VALUE\trailing text".to_vec()))]
    #[case(AstEntry::new_remove(br"KEY".to_vec(), br#"\"trailing text"#.to_vec()), IrEntry::new_remove(br"KEY".to_vec(), br#""trailing text"#.to_vec()))]
    fn parse_remove_ir_entry(#[case] ast: AstEntry, #[case] expected_ir: IrEntry) {
        let ir = IrEntry::from_ast(ast.clone());
        assert!(ir.is_ok(), "error converting AST to IR: {ast:?}");
        assert_eq!(ir.unwrap(), expected_ir);
    }

    #[rstest]
    #[case(AstEntry::new_reset(br"KEY".to_vec()), IrEntry::new_reset(br"KEY".to_vec()))]
    fn parse_reset_ir_entry(#[case] ast: AstEntry, #[case] expected_ir: IrEntry) {
        let ir = IrEntry::from_ast(ast.clone());
        assert!(ir.is_ok(), "error converting AST to IR: {ast:?}");
        assert_eq!(ir.unwrap(), expected_ir);
    }
}
