use crate::{Key, ext::Indent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFmt<'a> {
    key: Option<&'a Key>,
    increment_indent_by: Indent,
    indent: Indent,
    flatten: bool,
}

impl<'a> ConfigFmt<'a> {
    pub const fn new() -> Self {
        Self {
            key: None,
            indent: Indent::new(0),
            increment_indent_by: Indent::new(4),
            flatten: false,
        }
    }

    pub fn with_key(mut self, key: &'a Key) -> Self {
        self.key = Some(key);
        self
    }
    pub fn with_indent_increments(mut self, indent: Indent) -> Self {
        self.indent = indent;
        self
    }

    pub fn with_indent(mut self, indent: Indent) -> Self {
        self.indent = indent;
        self
    }

    pub fn with_flatten(mut self) -> Self {
        self.flatten = true;
        self
    }

    pub fn key(&self) -> Option<&'a Key> {
        self.key
    }

    pub fn indent(&self) -> Indent {
        self.indent
    }

    pub fn flatten(&self) -> bool {
        self.flatten
    }

    pub fn next(&self) -> Self {
        let mut next = self.clone();
        next.key = None;
        next.flatten = false;
        if !self.flatten {
            next.indent += self.increment_indent_by;
        }
        next
    }
}

impl<'a> Default for ConfigFmt<'a> {
    fn default() -> Self {
        Self::new()
    }
}
