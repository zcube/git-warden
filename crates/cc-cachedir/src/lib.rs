//! cc-cachedir: Build artifact and cache directory identification. Corresponds to Go `pkg/cachedir`.
//!
//! Verifies whether staged/tracked files are inside cache/build directories (node_modules, target, etc.)
//! based on parent directory indicators (go.mod, package.json, Cargo.toml, etc.).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Directory names that should never be traversed or modified. Corresponds to Go `ProtectedDirNames`.
pub fn is_protected_dir_name(name: &str) -> bool {
    matches!(name, ".git" | ".svn" | ".hg" | ".bzr")
}

// ── shared file lists ──
const JS_LOCKFILES: &[&str] = &[
    "package.json",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "bun.lockb",
    "bun.lock",
];
const GRADLE_FILES: &[&str] = &[
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "gradlew",
    "gradlew.bat",
];
const NUXT_CONFIGS: &[&str] = &["nuxt.config.js", "nuxt.config.ts", "nuxt.config.mjs"];
const NEXT_CONFIGS: &[&str] = &[
    "next.config.js",
    "next.config.ts",
    "next.config.mjs",
    "next.config.cjs",
];

/// Checks whether a candidate directory is an actual build artifact. Corresponds to Go `validator`.
enum Validator {
    /// A regular file matching one of `names` exists in the parent directory.
    ParentHasAny(&'static [&'static str]),
    /// The candidate directory itself contains a regular file matching one of `names`.
    DirContainsAny(&'static [&'static str]),
    /// The candidate directory itself contains a subdirectory matching one of `names`.
    DirContainsDirAny(&'static [&'static str]),
    /// A `.py` file exists in the parent of `__pycache__`.
    ParentHasPyFiles,
}

fn stat_any_in(base: &Path, names: &[&str], want_dir: bool) -> bool {
    for name in names {
        if let Ok(meta) = std::fs::metadata(base.join(name)) {
            if meta.is_dir() == want_dir {
                return true;
            }
        }
    }
    false
}

fn parent_has_py_files(dir_path: &Path) -> bool {
    let Some(parent) = dir_path.parent() else {
        return false;
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return false;
    };
    for e in entries.flatten() {
        if let Ok(ft) = e.file_type() {
            if !ft.is_dir() && e.file_name().to_string_lossy().ends_with(".py") {
                return true;
            }
        }
    }
    false
}

impl Validator {
    fn check(&self, dir_path: &Path) -> bool {
        match self {
            Validator::ParentHasAny(names) => match dir_path.parent() {
                Some(p) => stat_any_in(p, names, false),
                None => false,
            },
            Validator::DirContainsAny(names) => stat_any_in(dir_path, names, false),
            Validator::DirContainsDirAny(names) => stat_any_in(dir_path, names, true),
            Validator::ParentHasPyFiles => parent_has_py_files(dir_path),
        }
    }
}

/// Returns the list of validators for the given directory name. Corresponds to Go `dirValidators`.
/// At least one validator must pass for the directory to be recognised as a build artifact.
fn dir_validators(name: &str) -> Option<Vec<Validator>> {
    use Validator::*;
    let v = match name {
        "node_modules" => vec![ParentHasAny(JS_LOCKFILES)],
        "dist" => vec![
            ParentHasAny(JS_LOCKFILES),
            ParentHasAny(&["go.mod"]),
            ParentHasAny(&["Cargo.toml"]),
        ],
        "out" => vec![ParentHasAny(NEXT_CONFIGS), ParentHasAny(&["package.json"])],
        "build" => vec![
            ParentHasAny(&["package.json"]),
            ParentHasAny(&["Cargo.toml"]),
            ParentHasAny(GRADLE_FILES),
            ParentHasAny(&["pubspec.yaml"]),
            ParentHasAny(&["wails.json"]),
            ParentHasAny(&["CMakeLists.txt"]),
            DirContainsAny(&["CMakeCache.txt", "build.ninja"]),
        ],
        "target" => vec![
            ParentHasAny(&["Cargo.toml"]),
            ParentHasAny(&["pom.xml"]),
            ParentHasAny(&["build.sbt"]),
        ],
        "vendor" => vec![
            ParentHasAny(&["go.mod"]),
            ParentHasAny(&["Cargo.toml"]),
            ParentHasAny(&["composer.json"]),
            ParentHasAny(&["Gemfile"]),
            ParentHasAny(&["package.json"]),
        ],
        ".gradle" => vec![ParentHasAny(GRADLE_FILES)],
        ".next" => vec![ParentHasAny(NEXT_CONFIGS), ParentHasAny(&["package.json"])],
        ".nuxt" => vec![ParentHasAny(NUXT_CONFIGS)],
        ".output" => vec![ParentHasAny(NUXT_CONFIGS)],
        ".svelte-kit" => vec![ParentHasAny(&[
            "svelte.config.js",
            "svelte.config.ts",
            "svelte.config.cjs",
        ])],
        ".yarn" => vec![
            ParentHasAny(&["yarn.lock"]),
            ParentHasAny(&["package.json"]),
        ],
        ".bun" => vec![
            ParentHasAny(&["bun.lockb", "bun.lock"]),
            ParentHasAny(&["package.json"]),
        ],
        "__pycache__" => vec![
            ParentHasPyFiles,
            ParentHasAny(&[
                "pyproject.toml",
                "setup.py",
                "setup.cfg",
                "requirements.txt",
            ]),
        ],
        ".pytest_cache" => vec![ParentHasAny(&[
            "pytest.ini",
            "pyproject.toml",
            "setup.cfg",
            "tox.ini",
            "requirements.txt",
            "conftest.py",
        ])],
        ".mypy_cache" => vec![ParentHasAny(&[
            "mypy.ini",
            ".mypy.ini",
            "pyproject.toml",
            "setup.cfg",
        ])],
        ".ruff_cache" => vec![ParentHasAny(&["ruff.toml", "pyproject.toml"])],
        ".turbo" => vec![
            ParentHasAny(&["turbo.json"]),
            ParentHasAny(&["package.json"]),
        ],
        ".parcel-cache" => vec![ParentHasAny(&["package.json"])],
        ".venv" => vec![DirContainsAny(&["pyvenv.cfg"])],
        ".tox" => vec![ParentHasAny(&["tox.ini", "setup.cfg", "pyproject.toml"])],
        ".nox" => vec![ParentHasAny(&["noxfile.py"])],
        ".embuild" => vec![
            ParentHasAny(&["sdkconfig.defaults", "sdkconfig", "idf_component.yml"]),
            DirContainsDirAny(&["espressif"]),
        ],
        ".dart_tool" => vec![
            ParentHasAny(&["pubspec.yaml"]),
            DirContainsAny(&["package_config.json"]),
        ],
        _ => return None,
    };
    Some(v)
}

/// All cache/build directory names registered in `dirValidators`. Corresponds to Go `KnownCacheDirNames`.
pub const KNOWN_CACHE_DIR_NAMES: &[&str] = &[
    "node_modules",
    "dist",
    "out",
    "build",
    "target",
    "vendor",
    ".gradle",
    ".next",
    ".nuxt",
    ".output",
    ".svelte-kit",
    ".yarn",
    ".bun",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".turbo",
    ".parcel-cache",
    ".venv",
    ".tox",
    ".nox",
    ".embuild",
    ".dart_tool",
];

/// Returns true if `name` is a registered cache/build directory name. Corresponds to Go `IsKnownCacheDirName`.
pub fn is_known_cache_dir_name(name: &str) -> bool {
    dir_validators(name).is_some()
}

/// Returns all registered cache/build directory names. Corresponds to Go `KnownCacheDirNames`.
pub fn known_cache_dir_names() -> Vec<String> {
    KNOWN_CACHE_DIR_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn base_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Returns whether `dir_path` is an actual cache/build directory. Corresponds to Go `IsCacheDir`.
pub fn is_cache_dir(dir_path: &Path) -> bool {
    let name = base_name(dir_path);
    let Some(validators) = dir_validators(&name) else {
        return false;
    };
    validators.iter().any(|v| v.check(dir_path))
}

/// Returns whether `dir_path` is a Python virtualenv (contains pyvenv.cfg). Corresponds to Go `IsPythonVirtualenv`.
pub fn is_python_virtualenv(dir_path: &Path) -> bool {
    std::fs::metadata(dir_path.join("pyvenv.cfg")).is_ok()
}

/// Finds the nearest cache/build directory ancestor of `file_path`. Corresponds to Go `FindCacheDirAncestor`.
/// Does not traverse above `repo_root`.
pub fn find_cache_dir_ancestor(repo_root: &Path, file_path: &Path) -> Option<PathBuf> {
    let abs_root = abs(repo_root)?;
    let abs_file = abs(file_path)?;
    let sep = std::path::MAIN_SEPARATOR.to_string();
    let root_prefix = format!("{}{}", abs_root.to_string_lossy(), sep);

    let mut dir = abs_file.parent()?.to_path_buf();
    while dir != abs_root && dir.to_string_lossy().starts_with(&root_prefix) {
        if is_cache_dir(&dir) || is_python_virtualenv(&dir) {
            return Some(dir);
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent.to_path_buf(),
            _ => break,
        }
    }
    None
}

/// Finds all verified cache/build directories inside `repo_root`. Corresponds to Go `FindCacheDirsInRepo`.
pub fn find_cache_dirs_in_repo(repo_root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    visit(repo_root, &mut found);
    found
}

fn visit(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        let path = dir.join(&name);

        if is_protected_dir_name(&name) {
            continue;
        }
        // Detect virtualenvs by pyvenv.cfg (name-agnostic).
        if is_python_virtualenv(&path) {
            found.push(path);
            continue;
        }
        // Do not recurse into hidden directories not registered in dirValidators (.idea, .vscode, etc.).
        if name.starts_with('.') && !is_known_cache_dir_name(&name) {
            continue;
        }
        if is_known_cache_dir_name(&name) {
            if is_cache_dir(&path) {
                found.push(path);
            }
            // Do not recurse into known cache directories.
            continue;
        }
        visit(&path, found);
    }
}

fn abs(p: &Path) -> Option<PathBuf> {
    if p.is_absolute() {
        Some(normalize(p))
    } else {
        std::env::current_dir()
            .ok()
            .map(|cwd| normalize(&cwd.join(p)))
    }
}

// Path normalisation (strips . and ..): equivalent to filepath.Abs's Clean step.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        use std::path::Component::*;
        match comp {
            CurDir => {}
            ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn rel(base: &Path, target: &Path) -> Option<String> {
    let b = abs(base)?;
    let t = abs(target)?;
    let stripped = t.strip_prefix(&b).ok()?;
    Some(stripped.to_string_lossy().to_string())
}

/// Returns true if `targetDir` contains at least one untracked Git entry. Corresponds to Go `HasUntrackedEntries`.
pub fn has_untracked_entries(repo_root: &Path, target_dir: &Path) -> bool {
    let Some(rel) = rel(repo_root, target_dir) else {
        return false;
    };
    let Ok(out) = Command::new("git")
        .args([
            "-C",
            &repo_root.to_string_lossy(),
            "ls-files",
            "--others",
            "--directory",
            "-z",
            &rel,
        ])
        .output()
    else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    String::from_utf8_lossy(&out.stdout)
        .split('\u{0}')
        .any(|entry| !entry.trim_end_matches('/').is_empty())
}

/// Returns untracked files/directories inside `targetDir` as absolute paths. Corresponds to Go `ListUntrackedEntries`.
pub fn list_untracked_entries(repo_root: &Path, target_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let rel = rel(repo_root, target_dir).ok_or("relative path failed")?;
    let out = Command::new("git")
        .args([
            "-C",
            &repo_root.to_string_lossy(),
            "ls-files",
            "--others",
            "--directory",
            "-z",
            &rel,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("git ls-files failed".to_string());
    }
    let sep = std::path::MAIN_SEPARATOR.to_string();
    let prefix = format!("{rel}{sep}");
    let mut entries = Vec::new();
    for entry in String::from_utf8_lossy(&out.stdout).split('\u{0}') {
        let entry = entry.trim_end_matches('/');
        if entry.is_empty() {
            continue;
        }
        if entry != rel && !entry.starts_with(&prefix) {
            continue;
        }
        entries.push(repo_root.join(entry));
    }
    Ok(entries)
}

/// Returns tracked files inside `targetDir` as absolute paths. Corresponds to Go `ListTrackedEntries`.
pub fn list_tracked_entries(repo_root: &Path, target_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let rel = rel(repo_root, target_dir).ok_or("relative path failed")?;
    let out = Command::new("git")
        .args([
            "-C",
            &repo_root.to_string_lossy(),
            "ls-files",
            "-z",
            "--",
            &rel,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("git ls-files failed".to_string());
    }
    let mut entries = Vec::new();
    for entry in String::from_utf8_lossy(&out.stdout).split('\u{0}') {
        if entry.is_empty() {
            continue;
        }
        entries.push(repo_root.join(entry));
    }
    Ok(entries)
}

/// Returns the disk usage of `path` in bytes via `du -sk`. Returns 0 on failure. Corresponds to Go `GetDirSize`.
pub fn get_dir_size(path: &Path) -> i64 {
    let Ok(out) = Command::new("du").args(["-sk"]).arg(path).output() else {
        return 0;
    };
    if !out.status.success() {
        return 0;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let kb: i64 = s
        .split_whitespace()
        .next()
        .and_then(|t| t.parse().ok())
        .unwrap_or(0);
    kb * 1024
}

/// Formats a byte count in a human-readable unit. Corresponds to Go `FormatBytes`.
pub fn format_bytes(b: i64) -> String {
    const UNIT: i64 = 1024;
    if b < UNIT {
        return format!("{b} B");
    }
    let mut div: i64 = UNIT;
    let mut exp = 0usize;
    let mut n = b / UNIT;
    while n >= UNIT {
        div *= UNIT;
        exp += 1;
        n /= UNIT;
    }
    let units = ['K', 'M', 'G', 'T', 'P', 'E'];
    format!("{:.1} {}B", b as f64 / div as f64, units[exp])
}

/// Finds the Git repository root (directory containing .git) starting from `startDir`. Corresponds to Go `FindRepoRoot`.
pub fn find_repo_root(start_dir: &Path) -> Result<PathBuf, String> {
    let mut dir = abs(start_dir).ok_or("abs failed")?;
    loop {
        if std::fs::metadata(dir.join(".git")).is_ok() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent.to_path_buf(),
            _ => return Err(format!("not in a git repository: {}", start_dir.display())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn is_cache_dir_node_modules() {
        let dir = tempdir().unwrap();
        write_file(&dir.path().join("package.json"), "{}");
        let nm = dir.path().join("node_modules");
        fs::create_dir_all(&nm).unwrap();
        assert!(is_cache_dir(&nm));
    }

    #[test]
    fn is_cache_dir_node_modules_without_indicator() {
        let dir = tempdir().unwrap();
        let nm = dir.path().join("node_modules");
        fs::create_dir_all(&nm).unwrap();
        assert!(!is_cache_dir(&nm));
    }

    #[test]
    fn is_cache_dir_build_with_cargo() {
        let dir = tempdir().unwrap();
        write_file(&dir.path().join("Cargo.toml"), "");
        let b = dir.path().join("build");
        fs::create_dir_all(&b).unwrap();
        assert!(is_cache_dir(&b));
    }

    #[test]
    fn is_cache_dir_build_cmake_out_of_source() {
        let dir = tempdir().unwrap();
        let b = dir.path().join("build");
        write_file(&b.join("CMakeCache.txt"), "");
        assert!(is_cache_dir(&b));
    }

    #[test]
    fn is_cache_dir_unknown_name() {
        let dir = tempdir().unwrap();
        let other = dir.path().join("myrandomdir");
        fs::create_dir_all(&other).unwrap();
        assert!(!is_cache_dir(&other));
    }

    #[test]
    fn python_virtualenv() {
        let dir = tempdir().unwrap();
        let v = dir.path().join("myenv");
        write_file(&v.join("pyvenv.cfg"), "home = /usr/bin\n");
        assert!(is_python_virtualenv(&v));
    }

    #[test]
    fn find_cache_dir_ancestor_direct() {
        let root = tempdir().unwrap();
        write_file(&root.path().join("package.json"), "{}");
        let file = root.path().join("node_modules/lodash/index.js");
        write_file(&file, "");
        let anc = find_cache_dir_ancestor(root.path(), &file).expect("ancestor");
        assert_eq!(base_name(&anc), "node_modules");
    }

    #[test]
    fn find_cache_dir_ancestor_not_found() {
        let root = tempdir().unwrap();
        let file = root.path().join("src/main.go");
        write_file(&file, "package main");
        assert!(find_cache_dir_ancestor(root.path(), &file).is_none());
    }

    #[test]
    fn find_cache_dirs_in_repo_filters() {
        let root = tempdir().unwrap();
        write_file(&root.path().join("package.json"), "{}");
        fs::create_dir_all(root.path().join("node_modules/lib")).unwrap();
        fs::create_dir_all(root.path().join("subdir/build")).unwrap();
        write_file(&root.path().join("myenv/pyvenv.cfg"), "home = /usr/bin\n");

        let dirs = find_cache_dirs_in_repo(root.path());
        let names: Vec<String> = dirs.iter().map(|d| base_name(d)).collect();
        assert!(names.contains(&"node_modules".to_string()));
        assert!(names.contains(&"myenv".to_string()));
        assert!(!names.contains(&"build".to_string()));
    }

    #[test]
    fn find_cache_dirs_in_repo_stops_at_known() {
        let root = tempdir().unwrap();
        write_file(&root.path().join("package.json"), "{}");
        fs::create_dir_all(root.path().join("node_modules/some-pkg/dist")).unwrap();
        write_file(
            &root.path().join("node_modules/some-pkg/package.json"),
            "{}",
        );
        let dirs = find_cache_dirs_in_repo(root.path());
        assert!(dirs.iter().all(|d| base_name(d) != "dist"));
    }

    #[test]
    fn format_bytes_cases() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }
}
