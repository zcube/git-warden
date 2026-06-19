//! Ported tests from Go `internal/comment`. testdata paths are relative to the crate root.

use cc_comment::*;

fn classify(all: &[Comment]) -> (Vec<Comment>, Vec<Comment>, Vec<Comment>) {
    let mut comments = Vec::new();
    let mut strings = Vec::new();
    let mut imports = Vec::new();
    for c in all {
        match c.kind {
            Kind::Comment => comments.push(c.clone()),
            Kind::String => strings.push(c.clone()),
            Kind::Import => imports.push(c.clone()),
        }
    }
    (comments, strings, imports)
}

fn load_testdata(name: &str) -> String {
    std::fs::read_to_string(format!("testdata/{name}")).expect("testdata read")
}

fn has_comment(comments: &[Comment], substr: &str) -> bool {
    comments.iter().any(|c| c.text.contains(substr))
}

fn has_import(imports: &[Comment], path: &str) -> bool {
    imports.iter().any(|c| c.text == path)
}

// ---- parser_test.go ----

#[test]
fn go_parser_basic() {
    let src = "package main\n\n// 이것은 한국어 주석입니다\n// This is an English comment\n\n/* 블록 주석\n * 여러 줄\n */\n\nfunc main() {\n\t// nolint:errcheck\n\tx := \"// not a comment\"\n\t_ = x\n}\n";
    let comments = GoParser.parse_file(src).unwrap();
    assert!(!comments.is_empty());
    for c in &comments {
        assert_ne!(
            c.text, "not a comment",
            "string literal extracted as comment"
        );
    }
}

#[test]
fn cstyle_typescript_basic() {
    let src = "// 한국어 주석\nconst msg = \"// 이건 주석 아님\";\n/* block comment */\nconst tmpl = `hello // also not a comment`;\n";
    let p = CStyleParser::new(vec![".ts"], true);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 2, "{comments:?}");
}

#[test]
fn python_basic() {
    let src = "# 한국어 주석\nx = \"# 이건 주석 아님\"\ny = '# 이것도 아님'\nz = \"\"\"\ntriple quoted\n\"\"\"\n# another comment\n";
    let all = PythonParser.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 2, "{comments:?}");
}

// ---- parser_edge_test.go : Go ----

#[test]
fn go_build_tag_extracted() {
    let src = "//go:build linux\n\npackage main\n\n// 실제 주석입니다\nfunc main() {}\n";
    let comments = GoParser.parse_file(src).unwrap();
    assert!(comments.len() >= 2);
    assert!(comments.iter().any(|c| c.text.contains("실제 주석")));
}

#[test]
fn go_string_with_comment_syntax() {
    let src = "package main\n\nfunc main() {\n\tmsg := \"// this is not a comment\"\n\tother := \"/* also not a comment */\"\n\t_ = msg\n\t_ = other\n}\n";
    let all = GoParser.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    for c in &comments {
        assert!(!c.text.contains("this is not a comment"));
        assert!(!c.text.contains("also not a comment"));
    }
}

#[test]
fn go_block_multiline() {
    let src = "package main\n\n/*\n이것은 여러 줄\n블록 주석입니다\n*/\nfunc main() {}\n";
    let comments = GoParser.parse_file(src).unwrap();
    assert_eq!(comments.len(), 1);
    assert!(comments[0].end_line > comments[0].line);
    assert!(comments[0].is_block);
}

#[test]
fn go_line_numbers() {
    let src = "package main\n\n// line 3 comment\nfunc a() {}\n\n// line 6 comment\nfunc b() {}\n";
    let comments = GoParser.parse_file(src).unwrap();
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].line, 3);
    assert_eq!(comments[1].line, 6);
}

// ---- parser_edge_test.go : C-style ----

#[test]
fn cstyle_template_literal_not_comment() {
    let src = "const a = `hello // this is not a comment`;\n";
    let p = CStyleParser::new(vec![".ts"], true);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert!(!comments
        .iter()
        .any(|c| c.text.contains("this is not a comment")));
}

#[test]
fn cstyle_double_quote_not_comment() {
    let src = "const x = \"// not a comment\";\n// 실제 주석\n";
    let p = CStyleParser::new(vec![".ts"], true);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 1, "{comments:?}");
}

#[test]
fn cstyle_block_star_lines() {
    let src = "/**\n * 이 함수는 데이터를 처리합니다.\n * @param data 입력 데이터\n */\nfunction process(data) {}\n";
    let p = CStyleParser::new(vec![".ts"], true);
    let comments = p.parse_file(src).unwrap();
    assert_eq!(comments.len(), 1);
    assert!(!comments[0].text.contains("* 이"));
    assert!(comments[0].text.contains("이 함수는"));
}

#[test]
fn cstyle_escaped_quote_in_string() {
    let src = "const x = \"he said \\\"// not a comment\\\" here\";\n// 진짜 주석\n";
    let p = CStyleParser::new(vec![".ts"], true);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 1, "{comments:?}");
}

#[test]
fn cstyle_line_comment_after_code() {
    let src = "int x = 5; // 변수 x 초기화\n";
    let p = CStyleParser::new(vec![".go"], false);
    let comments = p.parse_file(src).unwrap();
    assert_eq!(comments.len(), 1);
    assert!(comments[0].text.contains("변수 x"));
}

#[test]
fn cstyle_java_multiple() {
    let src = "public class Main {\n    // 첫 번째 메서드\n    public void first() {\n        // 내부 주석\n    }\n\n    // 두 번째 메서드\n    public void second() {}\n}\n";
    let p = CStyleParser::new(vec![".java"], false);
    let comments = p.parse_file(src).unwrap();
    assert_eq!(comments.len(), 3, "{comments:?}");
}

#[test]
fn cstyle_rust_line_comment() {
    let src = "// 러스트 함수\nfn main() {\n    // 내부 주석\n    println!(\"hello\");\n}\n";
    let p = CStyleParser::new(vec![".rs"], false);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 2);
}

#[test]
fn cstyle_single_quote_not_comment() {
    let src = "const ch = '// not a comment';\n// 실제 주석\n";
    let p = CStyleParser::new(vec![".ts"], true);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert!(!comments.iter().any(|c| c.text.contains("not a comment")));
}

// ---- parser_edge_test.go : Python ----

#[test]
fn python_hash_in_string() {
    let src = "x = \"# not a comment\"\ny = '# also not'\n# 진짜 주석\n";
    let all = PythonParser.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 1, "{comments:?}");
    assert!(comments[0].text.contains("진짜 주석"));
}

#[test]
fn python_triple_quote_not_comment() {
    let src = "def foo():\n    \"\"\"\n    This docstring # is not a comment\n    \"\"\"\n    pass\n# 실제 주석\n";
    let all = PythonParser.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    for c in &comments {
        assert!(!c.text.contains("not a comment"));
    }
    assert!(comments.iter().any(|c| c.text.contains("실제 주석")));
}

#[test]
fn python_inline_comment() {
    let all = PythonParser.parse_file("x = 5  # 변수 초기화\n").unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].text.contains("변수 초기화"));
}

#[test]
fn python_multiple_comments() {
    let src =
        "# 첫 번째 주석\ndef foo():\n    # 두 번째 주석\n    x = 1  # 세 번째 주석\n    return x\n";
    let all = PythonParser.parse_file(src).unwrap();
    assert_eq!(all.len(), 3, "{all:?}");
}

#[test]
fn python_line_numbers() {
    let src = "x = 1\n# 두 번째 줄 주석\ny = 2\n# 네 번째 줄 주석\n";
    let all = PythonParser.parse_file(src).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].line, 2);
    assert_eq!(all[1].line, 4);
}

#[test]
fn python_pep723_basic() {
    let src = "#!/usr/bin/env -S uv run --script\n# /// script\n# dependencies = [\n#   \"ruamel.yaml>=0.18.0\",\n#   \"tabulate>=0.9.0\",\n# ]\n# ///\n# 진짜 주석입니다\nx = 1\n";
    let all = PythonParser.parse_file(src).unwrap();
    let (comments, _, imports) = classify(&all);
    assert_eq!(comments.len(), 1, "{comments:?}");
    assert!(comments[0].text.contains("진짜 주석"));
    assert!(!imports.is_empty());
}

#[test]
fn python_pep723_tooluv() {
    let src = "# /// tool.uv\n# constraint-dependencies = [\n#   \"requests>=2.28.0\",\n# ]\n# ///\n# 정상 주석\n";
    let all = PythonParser.parse_file(src).unwrap();
    let (comments, _, imports) = classify(&all);
    assert_eq!(comments.len(), 1, "{comments:?}");
    assert!(!imports.is_empty());
}

#[test]
fn python_pep723_multiple_blocks() {
    let src = "# /// script\n# requires-python = \">=3.11\"\n# ///\n# /// tool.uv\n# environments = [\"linux\"]\n# ///\n# 실제 주석\n";
    let all = PythonParser.parse_file(src).unwrap();
    let (comments, _, imports) = classify(&all);
    assert_eq!(comments.len(), 1, "{comments:?}");
    assert!(!imports.is_empty());
}

#[test]
fn python_pep723_not_false_positive() {
    let src = "# ///\n# /// 123-invalid type here with spaces\n# 일반 주석\n";
    let all = PythonParser.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 3, "{comments:?}");
}

#[test]
fn python_pep723_open_tag_line_number() {
    let src = "x = 1\n# /// script\n# deps = []\n# ///\ny = 2\n# 다섯 번째 줄 주석\n";
    let all = PythonParser.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 1, "{comments:?}");
    assert_eq!(comments[0].line, 6);
}

// ---- testdata_test.go ----

#[test]
fn testdata_go() {
    let all = GoParser.parse_file(&load_testdata("sample.go")).unwrap();
    let (comments, strings, imports) = classify(&all);
    assert!(has_comment(&comments, "패키지 수준 상수"));
    assert!(has_comment(&comments, "입력 값 유효성 검사"));
    assert!(has_comment(&comments, "블록 주석"));
    assert!(has_import(&imports, "fmt"));
    assert!(has_import(&imports, "strings"));
    assert!(has_import(&imports, "github.com/example/somepackage"));
    for imp in ["fmt", "strings", "github.com/example/somepackage"] {
        assert!(
            !strings.iter().any(|s| s.text == imp),
            "import leaked as string: {imp}"
        );
    }
}

#[test]
fn testdata_typescript() {
    let p = CStyleParser::new(vec![".ts"], true);
    let all = p.parse_file(&load_testdata("sample.ts")).unwrap();
    let (comments, strings, imports) = classify(&all);
    assert!(has_comment(&comments, "사용자 정보를 나타내는 인터페이스"));
    assert!(has_comment(&comments, "API에서 사용자 데이터를"));
    for imp in ["react", "axios", "reflect-metadata"] {
        assert!(!strings.iter().any(|s| s.text == imp));
        assert!(has_import(&imports, imp));
    }
}

#[test]
fn testdata_javascript() {
    let p = CStyleParser::new(vec![".js"], true);
    let all = p.parse_file(&load_testdata("sample.js")).unwrap();
    let (comments, strings, imports) = classify(&all);
    assert!(has_comment(&comments, "설정 파일을 불러오는 함수"));
    assert!(has_comment(&comments, "파일 내용을 읽어서"));
    for imp in ["path", "dotenv/config"] {
        assert!(!strings.iter().any(|s| s.text == imp));
        assert!(has_import(&imports, imp));
    }
}

#[test]
fn testdata_java() {
    let p = CStyleParser::new(vec![".java"], false);
    let all = p.parse_file(&load_testdata("sample.java")).unwrap();
    let (comments, _, imports) = classify(&all);
    assert!(has_comment(&comments, "사용자 서비스 클래스"));
    assert!(has_comment(&comments, "입력 유효성 검사 후 추가"));
    assert_eq!(imports.len(), 0);
}

#[test]
fn testdata_kotlin() {
    let p = CStyleParser::new(vec![".kt"], false);
    let all = p.parse_file(&load_testdata("sample.kt")).unwrap();
    let (comments, _, _) = classify(&all);
    assert!(has_comment(&comments, "아이템 저장소 인터페이스"));
    assert!(has_comment(&comments, "데이터베이스 기반 아이템 저장소"));
}

#[test]
fn testdata_python() {
    let all = PythonParser
        .parse_file(&load_testdata("sample.py"))
        .unwrap();
    let (comments, _, imports) = classify(&all);
    assert!(has_comment(&comments, "설정 파일 기본 경로"));
    assert!(has_comment(&comments, "경로가 지정되지 않으면"));
    assert_eq!(imports.len(), 0);
}

#[test]
fn testdata_c() {
    let p = CStyleParser::new(vec![".c"], false);
    let all = p.parse_file(&load_testdata("sample.c")).unwrap();
    let (comments, strings, imports) = classify(&all);
    assert!(has_comment(&comments, "프로그램의 진입점"));
    assert!(has_comment(&comments, "인자 수 확인"));
    assert!(has_import(&imports, "utils.h"));
    assert!(has_import(&imports, "config.h"));
    for imp in ["utils.h", "config.h"] {
        assert!(!strings.iter().any(|s| s.text == imp));
    }
}

#[test]
fn testdata_cpp() {
    let p = CStyleParser::new(vec![".cpp"], false);
    let all = p.parse_file(&load_testdata("sample.cpp")).unwrap();
    let (comments, strings, imports) = classify(&all);
    assert!(has_comment(&comments, "데이터 처리기 클래스"));
    assert!(has_comment(&comments, "생성자"));
    assert!(has_import(&imports, "processor.hpp"));
    assert!(!strings.iter().any(|s| s.text == "processor.hpp"));
}

#[test]
fn testdata_csharp() {
    let p = CStyleParser::new(vec![".cs"], false);
    let all = p.parse_file(&load_testdata("sample.cs")).unwrap();
    let (comments, _, _) = classify(&all);
    assert!(has_comment(&comments, "주문 처리 서비스"));
    assert!(has_comment(&comments, "주문 저장소"));
}

#[test]
fn testdata_swift() {
    let p = CStyleParser::new(vec![".swift"], false);
    let all = p.parse_file(&load_testdata("sample.swift")).unwrap();
    let (comments, _, _) = classify(&all);
    assert!(has_comment(&comments, "네트워크 요청을 관리하는 클래스"));
    assert!(has_comment(&comments, "공유 인스턴스"));
}

#[test]
fn testdata_rust() {
    let p = CStyleParser::new(vec![".rs"], false);
    let all = p.parse_file(&load_testdata("sample.rs")).unwrap();
    let (comments, _, _) = classify(&all);
    assert!(has_comment(&comments, "설정 구조체"));
    assert!(has_comment(&comments, "파일에서 설정을 불러옵니다"));
}

#[test]
fn testdata_dockerfile() {
    let all = DockerfileParser
        .parse_file(&load_testdata("Dockerfile"))
        .unwrap();
    let (comments, strings, _) = classify(&all);
    assert!(!comments.is_empty());
    assert!(has_comment(&comments, "빌드 스테이지"));
    assert!(has_comment(&comments, "의존성 파일을 먼저 복사"));
    assert!(has_comment(&comments, "실행 스테이지"));
    assert_eq!(strings.len(), 0);
}

#[test]
fn testdata_markdown() {
    let all = MarkdownParser
        .parse_file(&load_testdata("sample.md"))
        .unwrap();
    let (comments, _, _) = classify(&all);
    assert!(has_comment(&comments, "이것은 HTML 주석입니다"));
    assert!(has_comment(&comments, "이것은 마크다운 주석입니다"));
    for c in &comments {
        assert!(!c.text.contains("프로젝트 소개"));
        assert!(!c.text.contains("설치 방법"));
        assert!(!c.text.contains("go install"));
    }
}

#[test]
fn testdata_hcl() {
    let all = HclParser.parse_file(&load_testdata("sample.tf")).unwrap();
    let (comments, _, imports) = classify(&all);
    assert!(has_comment(&comments, "웹 서버 인스턴스 정의"));
    assert!(has_comment(&comments, "초기화 스크립트"));
    assert!(has_comment(&comments, "블록 주석"));
    for c in &comments {
        assert!(!c.text.contains("this hash inside heredoc"));
    }
    assert_eq!(imports.len(), 0);
}

// ---- get_parser ----

#[test]
fn get_parser_all_languages() {
    let cases: &[(&str, bool)] = &[
        ("main.go", false),
        ("app.ts", false),
        ("app.tsx", false),
        ("index.js", false),
        ("index.jsx", false),
        ("Main.java", false),
        ("Main.kt", false),
        ("script.py", false),
        ("main.c", false),
        ("main.cpp", false),
        ("Program.cs", false),
        ("App.swift", false),
        ("lib.rs", false),
        ("Dockerfile", false),
        ("Dockerfile.prod", false),
        ("Dockerfile.dev", false),
        ("app.dockerfile", false),
        ("README.md", false),
        ("docs.markdown", false),
        ("main.tf", false),
        ("terraform.tfvars", false),
        ("config.hcl", false),
        ("unknown.xyz", true),
    ];
    for (file, want_none) in cases {
        let p = get_parser(file);
        assert_eq!(p.is_none(), *want_none, "file={file}");
    }
}

// ---- registry_test.go ----

#[test]
fn extensions_for_languages_cases() {
    let go = extensions_for_languages(&["go".to_string()]);
    assert!(go.contains(&".go".to_string()));
    let ts = extensions_for_languages(&["typescript".to_string()]);
    assert!(ts.contains(&".ts".to_string()) && ts.contains(&".tsx".to_string()));
    let multi = extensions_for_languages(&["go".to_string(), "python".to_string()]);
    assert!(multi.contains(&".go".to_string()) && multi.contains(&".py".to_string()));
    assert!(extensions_for_languages(&["unknown_lang".to_string()]).is_empty());
}

// ---- html_parser_test.go ----

#[test]
fn html_single_line() {
    let src = "<html>\n<!-- This is a comment -->\n<body></body>\n</html>\n";
    let comments = HtmlParser.parse_file(src).unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].text, "This is a comment");
    assert!(comments[0].is_block);
}

#[test]
fn html_multi_line() {
    let src = "<html>\n<!--\n  Multi-line\n  comment here\n-->\n</html>\n";
    let comments = HtmlParser.parse_file(src).unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].line, 2);
    assert_eq!(comments[0].end_line, 5);
}

#[test]
fn html_multiple() {
    let src = "<!-- first -->\n<p>text</p>\n<!-- second -->\n";
    let comments = HtmlParser.parse_file(src).unwrap();
    assert_eq!(comments.len(), 2);
}

#[test]
fn html_no_comments() {
    let comments = HtmlParser
        .parse_file("<html><body><p>Hello</p></body></html>")
        .unwrap();
    assert_eq!(comments.len(), 0);
}

#[test]
fn html_kind_is_comment() {
    let comments = HtmlParser.parse_file("<!-- hello world -->").unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].kind, Kind::Comment);
}

// ---- hash_parser_test.go ----

#[test]
fn hash_shell() {
    let src = "#!/bin/bash\n# 이것은 한국어 주석입니다\n# This is an English comment\necho \"# not a comment\"\n# another comment\n";
    let p = HashStyleParser::new(vec![".sh"], false);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 3, "{comments:?}");
}

#[test]
fn hash_shell_no_ruby_blocks() {
    let src =
        "# normal comment\n=begin\nthis should not be a block comment\n=end\n# another comment\n";
    let p = HashStyleParser::new(vec![".sh"], false);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 2, "{comments:?}");
}

#[test]
fn hash_ruby_line_comment() {
    let src = "# 한국어 주석\nx = 1 # inline comment\n# another comment\n";
    let p = HashStyleParser::new(vec![".rb"], true);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    assert_eq!(comments.len(), 2, "{comments:?}");
}

#[test]
fn hash_ruby_block_comment() {
    let src = "# line comment\n=begin\nThis is a\nmulti-line block comment\n=end\n# after block\n";
    let p = HashStyleParser::new(vec![".rb"], true);
    let all = p.parse_file(src).unwrap();
    let (comments, _, _) = classify(&all);
    let line: Vec<_> = comments.iter().filter(|c| !c.is_block).collect();
    let block: Vec<_> = comments.iter().filter(|c| c.is_block).collect();
    assert_eq!(line.len(), 2);
    assert_eq!(block.len(), 1);
    assert_eq!(block[0].line, 2);
    assert_eq!(block[0].end_line, 5);
}

#[test]
fn hash_ruby_empty_block() {
    let p = HashStyleParser::new(vec![".rb"], true);
    let _ = p.parse_file("=begin\n=end\n").unwrap();
}

// ---- hcl_parser_test.go ----

fn hcl(src: &str) -> (Vec<Comment>, Vec<Comment>, Vec<Comment>) {
    classify(&HclParser.parse_file(src).unwrap())
}

#[test]
fn hcl_hash_line_comment() {
    let src = "# 해시 줄 주석입니다\nresource \"aws_instance\" \"web\" {\n  ami = \"ami-12345\" # 인라인 해시 주석\n}\n";
    let (comments, _, _) = hcl(src);
    assert_eq!(comments.len(), 2, "{comments:?}");
    assert_eq!(comments[0].text, "해시 줄 주석입니다");
    assert_eq!(comments[0].line, 1);
    assert_eq!(comments[1].text, "인라인 해시 주석");
    assert_eq!(comments[1].line, 3);
}

#[test]
fn hcl_slash_line_comment() {
    let src = "// 슬래시 줄 주석입니다\nvariable \"name\" {\n  default = \"value\" // 인라인 슬래시 주석\n}\n";
    let (comments, _, _) = hcl(src);
    assert_eq!(comments.len(), 2, "{comments:?}");
    assert_eq!(comments[0].text, "슬래시 줄 주석입니다");
    assert_eq!(comments[1].text, "인라인 슬래시 주석");
    assert_eq!(comments[1].line, 3);
}

#[test]
fn hcl_block_comment() {
    let src = "/* 블록 주석\n   두 번째 줄 */\nlocals {\n  x = 1\n}\n";
    let (comments, _, _) = hcl(src);
    assert_eq!(comments.len(), 1);
    assert!(comments[0].is_block);
    assert_eq!(comments[0].line, 1);
    assert_eq!(comments[0].end_line, 2);
    assert!(comments[0].text.contains("블록 주석") && comments[0].text.contains("두 번째 줄"));
}

#[test]
fn hcl_comment_markers_inside_string() {
    let src = "locals {\n  a = \"url#fragment\"\n  b = \"https://example.com\"\n  c = \"glob/*.txt\"\n}\n";
    let (comments, strings, _) = hcl(src);
    assert_eq!(comments.len(), 0, "{comments:?}");
    let want = ["url#fragment", "https://example.com", "glob/*.txt"];
    assert_eq!(strings.len(), want.len(), "{strings:?}");
    for (i, w) in want.iter().enumerate() {
        assert_eq!(strings[i].text, *w);
    }
}

#[test]
fn hcl_interpolation_nested_quotes() {
    let src = "locals {\n  v = \"x-${var.a == \"b\" ? 1 : 2}-y\" # 뒤쪽 주석\n}\n";
    let (comments, strings, _) = hcl(src);
    assert_eq!(strings.len(), 1, "{strings:?}");
    assert_eq!(strings[0].text, "x-${var.a == \"b\" ? 1 : 2}-y");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].text, "뒤쪽 주석");
    assert_eq!(comments[0].line, 2);
}

#[test]
fn hcl_interpolation_nested_braces() {
    let src = "locals {\n  v = \"a-${jsonencode({ key = \"v-${var.x}\" })}-z\"\n}\n";
    let (_, strings, _) = hcl(src);
    assert_eq!(strings.len(), 1, "{strings:?}");
    assert_eq!(
        strings[0].text,
        "a-${jsonencode({ key = \"v-${var.x}\" })}-z"
    );
}

#[test]
fn hcl_dollar_literal_escape() {
    let src = "locals {\n  a = \"literal $${not_interp} text\"\n  b = \"literal %%{not_directive} text\"\n}\n";
    let (_, strings, _) = hcl(src);
    assert_eq!(strings.len(), 2, "{strings:?}");
    assert_eq!(strings[0].text, "literal ${not_interp} text");
    assert_eq!(strings[1].text, "literal %{not_directive} text");
}

#[test]
fn hcl_template_directive() {
    let src =
        "locals {\n  v = \"%{ if var.env == \"prod\" }production%{ else }dev%{ endif }\"\n}\n";
    let (_, strings, _) = hcl(src);
    assert_eq!(strings.len(), 1, "{strings:?}");
    assert_eq!(
        strings[0].text,
        "%{ if var.env == \"prod\" }production%{ else }dev%{ endif }"
    );
}

#[test]
fn hcl_heredoc() {
    let src = "resource \"aws_instance\" \"web\" {\n  user_data = <<EOF\n#!/bin/bash\n# this hash is not a comment\necho \"hello\"\nEOF\n}\n";
    let (comments, strings, _) = hcl(src);
    assert_eq!(comments.len(), 0, "{comments:?}");
    assert_eq!(strings.len(), 3, "{strings:?}");
    let h = &strings[2];
    assert_eq!(
        h.text,
        "#!/bin/bash\n# this hash is not a comment\necho \"hello\""
    );
    assert_eq!(h.line, 2);
    assert_eq!(h.end_line, 6);
}

#[test]
fn hcl_heredoc_indented() {
    let src = "locals {\n  doc = <<-EOT\n    line one\n    line two\n  EOT\n}\n";
    let (_, strings, _) = hcl(src);
    assert_eq!(strings.len(), 1, "{strings:?}");
    let h = &strings[0];
    assert!(h.text.contains("line one") && h.text.contains("line two"));
    assert!(!h.text.contains("EOT"));
    assert_eq!(h.line, 2);
    assert_eq!(h.end_line, 5);
}

#[test]
fn hcl_heredoc_plain_allows_indented_label() {
    let src = "locals {\n  doc = <<EOT\nbody\n  EOT\nEOT\n}\n# 뒤따르는 주석\n";
    let (comments, strings, _) = hcl(src);
    assert_eq!(strings.len(), 1, "{strings:?}");
    assert_eq!(strings[0].text, "body");
    assert_eq!(strings[0].end_line, 4);
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].line, 7);
}

#[test]
fn hcl_line_numbers() {
    let src = "# 첫 줄 주석\n/* 블록\n   주석 */\nlocals {\n  a = \"문자열\" # 다섯째 줄 주석\n}\n// 일곱째 줄 주석\n";
    let (comments, strings, _) = hcl(src);
    assert_eq!(comments.len(), 4, "{comments:?}");
    let want = [(1, 1), (2, 3), (5, 5), (7, 7)];
    for (i, (l, el)) in want.iter().enumerate() {
        assert_eq!(comments[i].line, *l);
        assert_eq!(comments[i].end_line, *el);
    }
    assert_eq!(strings.len(), 1);
    assert_eq!(strings[0].line, 5);
}

#[test]
fn hcl_no_trailing_newline() {
    let (comments, _, _) = hcl("x = 1 # 마지막 주석");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].text, "마지막 주석");
}
