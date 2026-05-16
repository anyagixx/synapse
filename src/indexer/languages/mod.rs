// MODULE_CONTRACT
// MODULE_ID: M-INDEXER-LANGS
// PURPOSE: Language trait definitions and implementations for code parsing
// SCOPE: LanguageParser trait, 14 language structs (Rust, Python, TS, JS, Go, PHP, Cpp, Ruby, Java, Bash, Json, Css, Lua, Markdown), detect_language
// DEPENDS: N/A
// LINKS: N/A

// START_MODULE_MAP
// LanguageParser — Trait for language name and extensions
// Rust, Python, TypeScript, JavaScript, Go, Php, Cpp, Ruby, Java, Bash, Json, Css, Lua, Markdown — Language implementations
// detect_language — Detect language from file extension
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.0.0 — GRACE markup added]
// END_CHANGE_SUMMARY

// START_public_api

// START_LanguageParser
pub trait LanguageParser {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
}
// END_LanguageParser

pub struct Rust;
pub struct Python;
pub struct TypeScript;
pub struct JavaScript;
pub struct Go;
pub struct Php;
pub struct Cpp;
pub struct Ruby;
pub struct Java;
pub struct Bash;
pub struct Json;
pub struct Css;
pub struct Lua;
pub struct Markdown;

impl LanguageParser for Rust {
    fn name(&self) -> &'static str { "rust" }
    fn extensions(&self) -> &'static [&'static str] { &["rs"] }
}
impl LanguageParser for Python {
    fn name(&self) -> &'static str { "python" }
    fn extensions(&self) -> &'static [&'static str] { &["py"] }
}
impl LanguageParser for TypeScript {
    fn name(&self) -> &'static str { "typescript" }
    fn extensions(&self) -> &'static [&'static str] { &["ts", "tsx"] }
}
impl LanguageParser for JavaScript {
    fn name(&self) -> &'static str { "javascript" }
    fn extensions(&self) -> &'static [&'static str] { &["js", "jsx", "mjs", "cjs"] }
}
impl LanguageParser for Go {
    fn name(&self) -> &'static str { "go" }
    fn extensions(&self) -> &'static [&'static str] { &["go"] }
}
impl LanguageParser for Php {
    fn name(&self) -> &'static str { "php" }
    fn extensions(&self) -> &'static [&'static str] { &["php"] }
}
impl LanguageParser for Cpp {
    fn name(&self) -> &'static str { "cpp" }
    fn extensions(&self) -> &'static [&'static str] { &["cpp", "hpp", "h", "cc", "cxx"] }
}
impl LanguageParser for Ruby {
    fn name(&self) -> &'static str { "ruby" }
    fn extensions(&self) -> &'static [&'static str] { &["rb"] }
}
impl LanguageParser for Java {
    fn name(&self) -> &'static str { "java" }
    fn extensions(&self) -> &'static [&'static str] { &["java"] }
}
impl LanguageParser for Bash {
    fn name(&self) -> &'static str { "bash" }
    fn extensions(&self) -> &'static [&'static str] { &["sh", "bash", "zsh"] }
}
impl LanguageParser for Json {
    fn name(&self) -> &'static str { "json" }
    fn extensions(&self) -> &'static [&'static str] { &["json"] }
}
impl LanguageParser for Css {
    fn name(&self) -> &'static str { "css" }
    fn extensions(&self) -> &'static [&'static str] { &["css", "scss"] }
}
impl LanguageParser for Lua {
    fn name(&self) -> &'static str { "lua" }
    fn extensions(&self) -> &'static [&'static str] { &["lua"] }
}
impl LanguageParser for Markdown {
    fn name(&self) -> &'static str { "markdown" }
    fn extensions(&self) -> &'static [&'static str] { &["md"] }
}

// START_CONTRACT_detect_language
// PURPOSE: Detect language from file extension, returning a boxed LanguageParser
// INPUTS: { path: &Path — file path }
// OUTPUTS: { Option<Box<dyn LanguageParser>> }
// START_detect_language
pub fn detect_language(path: &std::path::Path) -> Option<Box<dyn LanguageParser>> {
    let ext = path.extension()?.to_str()?;
    let parsers: Vec<Box<dyn LanguageParser>> = vec![
        Box::new(Rust), Box::new(Python), Box::new(TypeScript),
        Box::new(JavaScript), Box::new(Go), Box::new(Php),
        Box::new(Cpp), Box::new(Ruby), Box::new(Java),
        Box::new(Bash), Box::new(Json), Box::new(Css),
        Box::new(Lua), Box::new(Markdown),
    ];
    for parser in parsers {
        if parser.extensions().contains(&ext) {
            return Some(parser);
        }
    }
    None
}
// END_detect_language
// END_public_api
