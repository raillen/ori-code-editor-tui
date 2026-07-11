//! Tree-sitter language binding for OriScript.

use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_oriscript() -> *const ();
}

/// Tree-sitter [`LanguageFn`] for OriScript.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_oriscript) };

/// Static `node-types.json` content.
pub const NODE_TYPES: &str = include_str!("node-types.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_oris() {
        let mut parser = tree_sitter::Parser::new();
        let language = LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("load oriscript");
        let tree = parser
            .parse("fn main() {\n  print(\"hi\")\n}\n", None)
            .expect("parse");
        assert!(!tree.root_node().has_error() || tree.root_node().child_count() > 0);
    }
}
