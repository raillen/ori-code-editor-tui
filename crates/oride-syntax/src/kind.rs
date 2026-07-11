//! Categorias semânticas de highlight.

/// Kind estável para mapear a cores de tema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HighlightKind {
    #[default]
    Normal,
    Comment,
    Keyword,
    String,
    Number,
    Type,
    Function,
    Operator,
    Punctuation,
    Variable,
    Constant,
    Property,
    Tag,
    Attribute,
}

impl HighlightKind {
    /// Mapeia `node.kind()` do tree-sitter → categoria.
    #[must_use]
    pub fn from_node_kind(node_kind: &str) -> Option<Self> {
        let k = node_kind;
        // OriScript tokens
        if matches!(k, "line_comment" | "block_comment" | "comment") {
            return Some(Self::Comment);
        }
        if matches!(
            k,
            "keyword"
                | "async"
                | "await"
                | "return"
                | "if"
                | "else"
                | "for"
                | "while"
                | "match"
                | "fn"
                | "let"
                | "const"
                | "struct"
                | "enum"
                | "module"
                | "use"
                | "pub"
                | "import"
                | "export"
                | "class"
                | "function"
                | "var"
                | "type"
                | "interface"
                | "extends"
                | "implements"
                | "new"
                | "this"
                | "super"
                | "try"
                | "catch"
                | "throw"
                | "break"
                | "continue"
                | "switch"
                | "case"
                | "default"
                | "do"
                | "in"
                | "of"
                | "from"
                | "as"
                | "with"
        ) {
            return Some(Self::Keyword);
        }
        if matches!(
            k,
            "type_builtin" | "type_identifier" | "primitive_type" | "type" | "predefined_type"
        ) {
            return Some(Self::Type);
        }
        if matches!(
            k,
            "constant_builtin" | "true" | "false" | "null" | "undefined" | "nil"
        ) {
            return Some(Self::Constant);
        }
        if matches!(
            k,
            "string"
                | "string_literal"
                | "string_fragment"
                | "string_content"
                | "template_string"
                | "raw_text"
                | "quoted_attribute_value"
                | "attribute_value"
        ) || k.contains("string")
        {
            return Some(Self::String);
        }
        if matches!(k, "number" | "integer" | "float" | "number_literal") || k.contains("number") {
            return Some(Self::Number);
        }
        if matches!(k, "operator" | "binary_operator" | "unary_operator") {
            return Some(Self::Operator);
        }
        if matches!(
            k,
            "punctuation" | "(" | ")" | "{" | "}" | "[" | "]" | ";" | "," | "." | ":" | "::"
        ) {
            return Some(Self::Punctuation);
        }
        if matches!(
            k,
            "identifier" | "variable" | "property_identifier" | "shorthand_property_identifier"
        ) {
            return Some(Self::Variable);
        }
        if matches!(k, "property" | "field_identifier" | "property_name") {
            return Some(Self::Property);
        }
        if matches!(k, "tag_name" | "start_tag" | "end_tag" | "self_closing_tag") {
            return Some(Self::Tag);
        }
        if matches!(k, "attribute" | "attribute_name") {
            return Some(Self::Attribute);
        }
        if matches!(
            k,
            "function" | "function_declaration" | "method_definition" | "call_expression"
        ) {
            return Some(Self::Function);
        }
        // Markdown
        if matches!(k, "atx_heading" | "setext_heading" | "heading_content") {
            return Some(Self::Keyword);
        }
        if matches!(k, "code_span" | "fenced_code_block" | "indented_code_block") {
            return Some(Self::String);
        }
        if matches!(k, "emphasis" | "strong_emphasis" | "strikethrough") {
            return Some(Self::Type);
        }
        if matches!(
            k,
            "link" | "image" | "uri_autolink" | "email_autolink" | "link_destination"
        ) {
            return Some(Self::Constant);
        }
        if k.ends_with("_comment") || k.contains("comment") {
            return Some(Self::Comment);
        }
        None
    }
}
