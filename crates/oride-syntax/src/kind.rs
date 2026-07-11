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
    // Markdown-oriented
    /// `#` headings / títulos
    Heading,
    /// *emphasis* / _italic_
    Emphasis,
    /// **strong**
    Strong,
    /// links e URLs
    Link,
    /// `code` e blocos de código
    Code,
    /// marcadores de lista / task
    ListMarker,
    /// block quote `>`
    Quote,
}

impl HighlightKind {
    /// Mapeia nome de capture de query tree-sitter (`@text.title`, etc.).
    #[must_use]
    pub fn from_capture_name(name: &str) -> Self {
        // Normaliza: "text.title" ou "punctuation.special"
        let n = name.trim_start_matches('@');
        match n {
            "comment" | "text.comment" | "spell" => Self::Comment,
            "keyword" | "keyword.operator" => Self::Keyword,
            "string" | "string.escape" | "text.literal" => Self::String,
            "number" => Self::Number,
            "type" | "type.builtin" => Self::Type,
            "function" | "function.builtin" => Self::Function,
            "operator" => Self::Operator,
            "punctuation"
            | "punctuation.delimiter"
            | "punctuation.special"
            | "punctuation.bracket" => Self::Punctuation,
            "variable" | "parameter" => Self::Variable,
            "constant" | "constant.builtin" | "boolean" => Self::Constant,
            "property" => Self::Property,
            "tag" | "tag.delimiter" => Self::Tag,
            "attribute" => Self::Attribute,
            "text.title" | "markup.heading" | "title" => Self::Heading,
            "text.emphasis" | "markup.italic" | "emphasis" => Self::Emphasis,
            "text.strong" | "markup.bold" | "strong" => Self::Strong,
            "text.uri" | "markup.link.url" | "text.reference" | "markup.link"
            | "markup.link.label" => Self::Link,
            "markup.raw" | "markup.raw.block" | "text.literal.block" => Self::Code,
            "markup.list" => Self::ListMarker,
            "markup.quote" | "text.quote" => Self::Quote,
            "none" => Self::Normal,
            other if other.contains("heading") || other.contains("title") => Self::Heading,
            other if other.contains("uri") || other.contains("link") => Self::Link,
            other if other.contains("emphasis") => Self::Emphasis,
            other if other.contains("strong") || other.contains("bold") => Self::Strong,
            other
                if other.contains("literal") || other.contains("code") || other.contains("raw") =>
            {
                Self::Code
            }
            other if other.contains("quote") => Self::Quote,
            other if other.contains("list") => Self::ListMarker,
            other if other.contains("comment") => Self::Comment,
            other if other.contains("string") => Self::String,
            other if other.contains("keyword") => Self::Keyword,
            _ => Self::Normal,
        }
    }

    /// Mapeia `node.kind()` do tree-sitter → categoria (fallback sem query).
    #[must_use]
    pub fn from_node_kind(node_kind: &str) -> Option<Self> {
        let k = node_kind;
        // --- Markdown (block + inline) ---
        if matches!(
            k,
            "atx_heading"
                | "setext_heading"
                | "heading_content"
                | "atx_h1_marker"
                | "atx_h2_marker"
                | "atx_h3_marker"
                | "atx_h4_marker"
                | "atx_h5_marker"
                | "atx_h6_marker"
                | "setext_h1_underline"
                | "setext_h2_underline"
        ) {
            return Some(Self::Heading);
        }
        if matches!(
            k,
            "code_span"
                | "code_span_delimiter"
                | "fenced_code_block"
                | "indented_code_block"
                | "code_fence_content"
                | "fenced_code_block_delimiter"
                | "info_string"
                | "language"
        ) {
            return Some(Self::Code);
        }
        if matches!(k, "emphasis" | "emphasis_delimiter") {
            return Some(Self::Emphasis);
        }
        if matches!(k, "strong_emphasis") {
            return Some(Self::Strong);
        }
        if matches!(k, "strikethrough") {
            return Some(Self::Comment);
        }
        if matches!(
            k,
            "link"
                | "inline_link"
                | "full_reference_link"
                | "collapsed_reference_link"
                | "shortcut_link"
                | "image"
                | "uri_autolink"
                | "email_autolink"
                | "link_destination"
                | "link_label"
                | "link_text"
                | "link_title"
                | "image_description"
                | "link_reference_definition"
        ) {
            return Some(Self::Link);
        }
        if matches!(
            k,
            "list_marker_plus"
                | "list_marker_minus"
                | "list_marker_star"
                | "list_marker_dot"
                | "list_marker_parenthesis"
                | "task_list_marker_checked"
                | "task_list_marker_unchecked"
        ) {
            return Some(Self::ListMarker);
        }
        if matches!(k, "block_quote" | "block_quote_marker") {
            return Some(Self::Quote);
        }
        if matches!(
            k,
            "pipe_table"
                | "pipe_table_header"
                | "pipe_table_row"
                | "pipe_table_cell"
                | "pipe_table_delimiter_row"
                | "pipe_table_delimiter_cell"
                | "pipe_table_align_left"
                | "pipe_table_align_right"
        ) {
            return Some(Self::Property);
        }
        if matches!(
            k,
            "html_block" | "html_tag" | "minus_metadata" | "plus_metadata"
        ) {
            return Some(Self::Tag);
        }
        if matches!(
            k,
            "backslash_escape" | "hard_line_break" | "entity_reference"
        ) {
            return Some(Self::Operator);
        }

        // --- Genérico / código ---
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
        ) {
            return Some(Self::Keyword);
        }
        if matches!(
            k,
            "type_builtin" | "type_identifier" | "primitive_type" | "predefined_type"
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
        if k.ends_with("_comment") || k.contains("comment") {
            return Some(Self::Comment);
        }
        None
    }
}
