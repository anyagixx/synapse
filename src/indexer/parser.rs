// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-PARSER
// PURPOSE: Tree-sitter AST parser — extracts code blocks (functions, structs, classes) with fallback
// SCOPE: ParserEngine, CodeBlock, tree-sitter parsing for Rust/Python/JS/TS/Go, regex fallback
// DEPENDS: N/A (tree-sitter grammars loaded at runtime)
// LINKS: N/A

// START_MODULE_MAP
// CodeBlock — Extracted code block with name, kind, line range, content
// ParserEngine — Tree-sitter based code block parser with multi-language support
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

use tree_sitter::{Language, Parser};

// START_public_api

// START_CodeBlock
pub struct CodeBlock {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
}
// END_CodeBlock

// START_ParserEngine
pub struct ParserEngine;
// END_ParserEngine

impl Default for ParserEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserEngine {
    // START_CONTRACT_ParserEngine::new
    // PURPOSE: Create a new ParserEngine
    // OUTPUTS: { Self }
    // START_parser_engine_new
    pub fn new() -> Self {
        Self
    }
    // END_parser_engine_new

    fn get_language(&self, lang: &str) -> Option<Language> {
        Some(match lang {
            "rust" => Language::new(tree_sitter_rust::LANGUAGE),
            "python" => Language::new(tree_sitter_python::LANGUAGE),
            "javascript" => Language::new(tree_sitter_javascript::LANGUAGE),
            "typescript" => Language::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
            "go" => Language::new(tree_sitter_go::LANGUAGE),
            _ => return None,
        })
    }

    // START_CONTRACT_ParserEngine::parse
    // PURPOSE: Parse source code into CodeBlocks using tree-sitter (with fallback)
    // INPUTS: { code: &str — source code }, { lang: &str — language identifier }
    // OUTPUTS: { Vec<CodeBlock> — extracted code blocks }
    // START_parser_engine_parse
    pub fn parse(&self, code: &str, lang: &str) -> Vec<CodeBlock> {
        let language = match self.get_language(lang) {
            Some(l) => l,
            None => return fallback_parse(code),
        };

        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            return fallback_parse(code);
        }

        let tree = match parser.parse(code, None) {
            Some(t) => t,
            None => return fallback_parse(code),
        };

        let mut blocks = Vec::new();
        let mut cursor = tree.walk();
        Self::extract_blocks(&mut cursor, code, "", &mut blocks);

        if blocks.is_empty() {
            return fallback_parse(code);
        }
        blocks
    }
    // END_parser_engine_parse

    fn extract_blocks(
        cursor: &mut tree_sitter::TreeCursor,
        code: &str,
        parent_name: &str,
        blocks: &mut Vec<CodeBlock>,
    ) {
        loop {
            let node = cursor.node();
            let kind = node.kind();

            let is_def = matches!(
                kind,
                "function_item"
                    | "function_definition"
                    | "method_definition"
                    | "class_definition"
                    | "struct_item"
                    | "trait_item"
                    | "impl_item"
                    | "enum_item"
                    | "type_item"
                    | "function"
                    | "method"
                    | "class_declaration"
            );

            if is_def {
                let name =
                    Self::node_name(node, code).unwrap_or_else(|| format!("unnamed_{}", kind));

                let full_name = if parent_name.is_empty() {
                    name
                } else {
                    format!("{}::{}", parent_name, name)
                };

                let start = node.start_position().row + 1;
                let end = node.end_position().row + 1;
                let content = node.utf8_text(code.as_bytes()).unwrap_or("").to_string();

                blocks.push(CodeBlock {
                    name: full_name.clone(),
                    kind: kind.to_string(),
                    start_line: start,
                    end_line: end,
                    content,
                });

                if cursor.goto_first_child() {
                    Self::extract_blocks(cursor, code, &full_name, blocks);
                    cursor.goto_parent();
                }
            } else {
                if cursor.goto_first_child() {
                    Self::extract_blocks(cursor, code, parent_name, blocks);
                    cursor.goto_parent();
                }
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    fn node_name(node: tree_sitter::Node, code: &str) -> Option<String> {
        if let Some(name_node) = node.child_by_field_name("name") {
            return name_node
                .utf8_text(code.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
        for i in 0..node.child_count() {
            let child = node.child(i)?;
            if child.kind() == "identifier" {
                return child.utf8_text(code.as_bytes()).ok().map(|s| s.to_string());
            }
        }
        None
    }
}

fn fallback_parse(code: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = code.lines().collect();

    let patterns: &[(&str, &str)] = &[
        // Generic (cross-language)
        ("fn ", "function"),
        ("pub fn ", "function"),
        ("pub async fn ", "async_function"),
        ("def ", "function"),
        ("class ", "class"),
        ("struct ", "struct"),
        ("enum ", "enum"),
        ("trait ", "trait"),
        ("impl ", "impl_block"),
        ("function ", "function"),
        ("export function ", "export_function"),
        ("export class ", "export_class"),
        ("func ", "function"),
        ("public class ", "class"),
        ("interface ", "interface"),
        ("module ", "module"),
        ("import ", "import_block"),
        // PHP
        ("public function ", "public_method"),
        ("protected function ", "protected_method"),
        ("private function ", "private_method"),
        ("abstract class ", "abstract_class"),
        // C/C++
        ("void ", "c_function"),
        ("int ", "c_function"),
        ("bool ", "c_function"),
        ("auto ", "c_function"),
        ("size_t ", "c_function"),
        ("string ", "c_function"),
        // Java
        ("public static void ", "java_static_method"),
        ("private void ", "private_method"),
        ("protected void ", "protected_method"),
        // Ruby
        ("def self.", "class_method"),
        ("attr_accessor ", "attribute"),
        // Bash
        ("function ", "bash_function"),
        // Lua
        ("local function ", "local_function"),
        // CSS/SCSS
        (".", "css_selector"),
        ("#", "css_id"),
    ];

    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        let mut matched = false;

        for &(prefix, kind) in patterns {
            if trimmed.starts_with(prefix) {
                let name = if kind == "css_selector" || kind == "css_id" {
                    trimmed
                        .trim_start_matches(prefix)
                        .split(&['{', ' ', ',', ':'] as &[_])
                        .next()
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    trimmed
                        .split(['(', '{', ':'])
                        .next()
                        .unwrap_or(trimmed)
                        .split_whitespace()
                        .last()
                        .unwrap_or("unknown")
                        .to_string()
                };

                // Find end of this block: next top-level pattern or EOF
                let end = lines[i..]
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find(|(_, l)| {
                        let t = l.trim();
                        patterns
                            .iter()
                            .any(|(p, _)| t.starts_with(p) && p != &"." && p != &"#")
                    })
                    .map(|(idx, _)| i + idx)
                    .unwrap_or(lines.len());

                let content = lines[i..end].join("\n");
                blocks.push(CodeBlock {
                    name,
                    kind: kind.to_string(),
                    start_line: i + 1,
                    end_line: end,
                    content,
                });
                i = end;
                matched = true;
                break;
            }
        }

        if !matched {
            i += 1;
        }
    }

    if blocks.is_empty() {
        return basic_fallback(code);
    }
    blocks
}

fn basic_fallback(code: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let patterns: &[(&str, &str)] = &[
        ("fn ", "function"),
        ("pub fn ", "function"),
        ("def ", "function"),
        ("class ", "class"),
        ("struct ", "struct"),
        ("enum ", "enum"),
        ("trait ", "trait"),
        ("impl ", "impl_block"),
        ("function ", "function"),
        ("export function ", "function"),
        ("export class ", "class"),
        ("func ", "function"),
        ("public class ", "class"),
    ];
    for (i, line) in code.lines().enumerate() {
        let trimmed = line.trim();
        if !patterns.iter().any(|(k, _)| trimmed.starts_with(k)) {
            continue;
        }
        let name = trimmed
            .split(['(', '{', ':'])
            .next()
            .unwrap_or(trimmed)
            .split_whitespace()
            .last()
            .unwrap_or("unknown")
            .to_string();
        let end = code
            .lines()
            .skip(i)
            .enumerate()
            .skip(1)
            .find(|(_, l)| patterns.iter().any(|(k, _)| l.trim().starts_with(k)))
            .map(|(idx, _)| i + idx)
            .unwrap_or(code.lines().count());
        let content = code
            .lines()
            .skip(i)
            .take(end - i)
            .collect::<Vec<_>>()
            .join("\n");
        blocks.push(CodeBlock {
            name,
            kind: "declaration".into(),
            start_line: i + 1,
            end_line: end,
            content,
        });
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_fn() {
        let engine = ParserEngine::new();
        let code =
            "fn hello() {\n    println!(\"hi\");\n}\n\nfn world() {\n    println!(\"there\");\n}";
        let blocks = engine.parse(code, "rust");
        assert!(!blocks.is_empty());
        assert!(blocks.iter().any(|b| b.name.contains("hello")));
        assert!(blocks.iter().any(|b| b.name.contains("world")));
    }

    #[test]
    fn test_parse_python_def() {
        let engine = ParserEngine::new();
        let code = "def greet(name):\n    return f\"Hello {name}\"";
        let blocks = engine.parse(code, "python");
        assert!(!blocks.is_empty());
        assert!(blocks[0].name.contains("greet"));
    }

    #[test]
    fn test_parse_js_function() {
        let engine = ParserEngine::new();
        let code = "function hello() {\n  console.log('hi');\n}";
        let blocks = engine.parse(code, "javascript");
        assert!(!blocks.is_empty());
        assert!(blocks.iter().any(|b| b.name.contains("hello")));
    }

    #[test]
    fn test_fallback_for_unsupported_language() {
        let engine = ParserEngine::new();
        let code =
            "function greet() {\n  return 'hello';\n}\n\nclass User {\n  constructor(name) {}\n}";
        let blocks = engine.parse(code, "php"); // PHP falls back
        assert!(!blocks.is_empty());
        assert!(blocks.iter().any(|b| b.name == "greet"));
    }

    #[test]
    fn test_fallback_empty_code() {
        let engine = ParserEngine::new();
        let blocks = engine.parse("", "rust");
        assert!(blocks.is_empty());
    }
}
// END_public_api
