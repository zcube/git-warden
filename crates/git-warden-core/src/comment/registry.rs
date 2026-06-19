//! Parser registry: file path → parser mapping and language → extension mapping. Corresponds to Go `registry.go`.

use super::*;
use std::path::Path;

/// Returns true if path is a Dockerfile / Dockerfile.* / *.dockerfile. Corresponds to Go `isDockerfilePath`.
pub fn is_dockerfile_path(path: &str) -> bool {
    let base = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let lower = base.to_lowercase();
    lower == "dockerfile" || lower.starts_with("dockerfile.") || lower.ends_with(".dockerfile")
}

// Extensions are checked in globalParsers order (same as Go init order).
fn global_parsers() -> Vec<Box<dyn Parser>> {
    vec![
        Box::new(GoParser),
        // JS/TS: handle backtick template literals.
        Box::new(CStyleParser::new(
            vec![".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"],
            true,
        )),
        // C family, JVM languages, PHP.
        Box::new(CStyleParser::new(
            vec![
                ".java", ".kt", ".kts", ".c", ".cpp", ".cc", ".cxx", ".h", ".hpp", ".cs", ".swift",
                ".rs", ".php", ".phtml",
            ],
            false,
        )),
        Box::new(PythonParser),
        Box::new(DockerfileParser),
        Box::new(MarkdownParser),
        // Ruby: includes =begin/=end block comments.
        Box::new(HashStyleParser::new(vec![".rb", ".rake", ".gemspec"], true)),
        // Shell: # line comments only.
        Box::new(HashStyleParser::new(
            vec![".sh", ".bash", ".zsh", ".fish", ".ksh"],
            false,
        )),
        Box::new(HtmlParser),
        Box::new(HclParser),
    ]
}

/// Language name → file extension mapping. Corresponds to Go `languageExtensions`.
fn language_extensions(lang: &str) -> &'static [&'static str] {
    match lang {
        "go" => &[".go"],
        "typescript" => &[".ts", ".tsx"],
        "javascript" => &[".js", ".jsx", ".mjs", ".cjs"],
        "java" => &[".java"],
        "kotlin" => &[".kt", ".kts"],
        "python" => &[".py"],
        "c" => &[".c", ".h"],
        "cpp" => &[".cpp", ".cc", ".cxx", ".hpp"],
        "csharp" => &[".cs"],
        "swift" => &[".swift"],
        "rust" => &[".rs"],
        "dockerfile" => &["dockerfile"],
        "markdown" => &[".md", ".markdown"],
        "ruby" => &[".rb", ".rake", ".gemspec"],
        "shell" => &[".sh", ".bash", ".zsh", ".fish", ".ksh"],
        "php" => &[".php", ".phtml"],
        "html" => &[".html", ".htm", ".svg"],
        "hcl" => &[".hcl", ".tf", ".tfvars"],
        _ => &[],
    }
}

/// Returns the union of file extensions for the given language names. Unknown languages are ignored. Corresponds to Go `ExtensionsForLanguages`.
pub fn extensions_for_languages(langs: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for lang in langs {
        for ext in language_extensions(lang) {
            if seen.insert(*ext) {
                result.push(ext.to_string());
            }
        }
    }
    result
}

/// Returns the Parser for the given path, or None if unsupported. Corresponds to Go `GetParser`.
pub fn get_parser(path: &str) -> Option<Box<dyn Parser>> {
    if is_dockerfile_path(path) {
        return Some(Box::new(DockerfileParser));
    }
    let ext = match Path::new(path).extension() {
        Some(e) => format!(".{}", e.to_string_lossy()),
        None => return None,
    };
    global_parsers()
        .into_iter()
        .find(|p| p.supported_extensions().contains(&ext.as_str()))
}
