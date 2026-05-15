use tree_sitter::{Parser, Language};

pub struct CodeBlock {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
}

pub struct ParserEngine;

impl ParserEngine {
    pub fn new() -> Self {
        Self
    }

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

    fn extract_blocks(
        cursor: &mut tree_sitter::TreeCursor,
        code: &str,
        parent_name: &str,
        blocks: &mut Vec<CodeBlock>,
    ) {
        loop {
            let node = cursor.node();
            let kind = node.kind();

            let is_def = matches!(kind,
                "function_item" | "function_definition" | "method_definition"
                | "class_definition" | "struct_item" | "trait_item"
                | "impl_item" | "enum_item" | "type_item"
                | "function" | "method" | "class_declaration"
            );

            if is_def {
                let name = Self::node_name(node, code)
                    .unwrap_or_else(|| format!("unnamed_{}", kind));

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
            return name_node.utf8_text(code.as_bytes()).ok().map(|s| s.to_string());
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
    let keywords: &[&str] = &[
        "fn ", "pub fn ", "pub async fn ", "async fn ",
        "def ", "class ", "struct ", "enum ", "trait ",
        "impl ", "function ", "export function ",
        "export async function ", "export class ",
        "func ", "public class ",
    ];
    for (i, line) in code.lines().enumerate() {
        let trimmed = line.trim();
        if !keywords.iter().any(|k| trimmed.starts_with(k)) {
            continue;
        }
        let name = trimmed.split(|c: char| c == '(' || c == '{' || c == ':')
            .next()
            .unwrap_or(trimmed)
            .trim()
            .split_whitespace()
            .last()
            .unwrap_or("unknown")
            .to_string();
        let end = code.lines().skip(i).enumerate().skip(1)
            .find(|(_, l)| keywords.iter().any(|k| l.trim().starts_with(k)))
            .map(|(idx, _)| i + idx)
            .unwrap_or(code.lines().count());
        let content = code.lines().skip(i).take(end - i).collect::<Vec<_>>().join("\n");
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
