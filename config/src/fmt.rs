use crate::ext::Indent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFmt {
    increment_indent_by: Indent,
    indent: Indent,
    flatten: bool,
}

impl ConfigFmt {
    pub const fn new() -> Self {
        Self {
            indent: Indent::new(0),
            increment_indent_by: Indent::new(4),
            flatten: false,
        }
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

    pub fn indent(&self) -> Indent {
        self.indent
    }

    pub fn flatten(&self) -> bool {
        self.flatten
    }

    pub fn next(&self) -> Self {
        let mut next = self.clone();
        next.flatten = false;
        if !self.flatten {
            next.indent += self.increment_indent_by;
        }
        next
    }
}

impl Default for ConfigFmt {
    fn default() -> Self {
        Self::new()
    }
}
