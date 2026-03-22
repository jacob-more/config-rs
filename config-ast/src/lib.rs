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
        Self { entries: Vec::new() }
    }
}

impl Default for AstTree {
    fn default() -> Self {
        Self::new()
    }
}

impl AstTree {
    pub fn parse_from_reader(reader: impl std::io::Read) {}
}

impl FromIterator<AstEntry> for AstTree {
    fn from_iter<T: IntoIterator<Item = AstEntry>>(entries: T) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}
