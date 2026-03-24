use bytes::Bytes;

pub mod parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstTree {
    entries: Vec<AstEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstEntry(ImplAstEntry);

#[derive(Debug, Clone, PartialEq, Eq)]
enum ImplAstEntry {
    Group {
        name: Bytes,
        entries: Vec<AstEntry>,
    },
    KeyOpValue {
        key: Bytes,
        operator: Bytes,
        value: Bytes,
    },
}

impl AstTree {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl Default for AstTree {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<AstEntry> for AstTree {
    fn from_iter<T: IntoIterator<Item = AstEntry>>(entries: T) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

impl AstEntry {
    fn new_key_value(
        key: impl Into<Bytes>,
        operator: impl Into<Bytes>,
        value: impl Into<Bytes>,
    ) -> Self {
        Self(ImplAstEntry::KeyOpValue {
            key: key.into(),
            operator: operator.into(),
            value: value.into(),
        })
    }

    pub fn new_group(key: impl Into<Bytes>, values: impl IntoIterator<Item = AstEntry>) -> Self {
        Self(ImplAstEntry::Group {
            name: key.into(),
            entries: values.into_iter().collect(),
        })
    }

    pub fn new_assign(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::new_key_value(key, Bytes::from_static(b"="), value)
    }

    pub fn new_assign_if_undefined(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::new_key_value(key, Bytes::from_static(b":="), value)
    }

    pub fn new_add(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::new_key_value(key, Bytes::from_static(b"+="), value)
    }

    pub fn new_remove(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::new_key_value(key, Bytes::from_static(b"-="), value)
    }

    pub fn new_reset(key: impl Into<Bytes>) -> Self {
        Self::new_key_value(key, Bytes::from_static(b"!"), Bytes::new())
    }
}
