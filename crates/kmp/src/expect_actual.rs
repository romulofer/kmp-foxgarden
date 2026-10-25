//! `expect`/`actual` navigation (`PLAN.md` Track 30 Phase 2).
//!
//! A multiplatform module declares an API once with `expect` in common code
//! and fulfils it with an `actual` in each platform's source set. Both
//! modifiers are syntactic, so declarations are found by reading a line's
//! modifiers and name rather than by running a compiler, and matched by
//! name (and a function's parameter types when its list sits on one line, so
//! overloads are told apart); a name left with several matches lists them all
//! and the picker lets the user choose. The scan reads every source file of the module, which
//! is why it answers a command and never a frame.

use std::path::{Path, PathBuf};

use fg_extension::{Counterpart, FileDiagnostic, ProblemSeverity};

use crate::source_set::{SOURCE_DIRS, locate};
use crate::tasks::{build_file_text, covered, targets};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Expect,
    Actual,
}

#[derive(Debug, PartialEq, Eq)]
struct Declaration {
    role: Role,
    name: String,
    /// Byte offset of `name` in the text it was found in.
    name_byte: usize,
    /// A function's parameter types, whitespace removed, when its parameter
    /// list closes on the same line. `None` for anything else, which is then
    /// matched by name alone.
    parameters: Option<Vec<String>>,
}

impl Declaration {
    /// Whether `self` and `other` can be an `expect`/`actual` pair: the same
    /// name, and the same parameter types when both are known, so overloads
    /// are told apart.
    fn pairs_with(&self, other: &Declaration) -> bool {
        self.name == other.name && !matches!((&self.parameters, &other.parameters), (Some(a), Some(b)) if a != b)
    }
}

const MODIFIERS: [&str; 24] = [
    "public", "private", "internal", "protected", "open", "abstract", "sealed", "data", "inline", "value", "enum",
    "annotation", "companion", "inner", "suspend", "operator", "infix", "tailrec", "external", "const", "lateinit",
    "override", "final", "fun",
];

const KEYWORDS: [&str; 6] = ["fun", "class", "interface", "object", "val", "var"];

/// Every `expect`/`actual` declaration in `source`.
fn declarations(source: &str) -> Vec<Declaration> {
    let mut found = Vec::new();
    let mut line_start = 0;
    for line in source.split_inclusive('\n') {
        if let Some(declaration) = declaration_in(line, line_start) {
            found.push(declaration);
        }
        line_start += line.len();
    }
    found
}

/// The declaration `line` starts, if its modifiers include `expect` or
/// `actual`. `line_start` is the line's byte offset in the file.
fn declaration_in(line: &str, line_start: usize) -> Option<Declaration> {
    let mut role = None;
    let mut rest = line.trim_start();
    loop {
        let word_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let word = &rest[..word_end];
        let after = rest[word_end..].trim_start();
        match word {
            "expect" => role = Some(Role::Expect),
            "actual" => role = Some(Role::Actual),
            _ if word.starts_with('@') => {}
            "typealias" => return named(line, line_start, role?, after, false),
            _ if KEYWORDS.contains(&word) && role.is_some() => {
                // `fun interface Name`: the keyword is `interface`.
                let interface = if word == "fun" { after.strip_prefix("interface") } else { None };
                let is_function = word == "fun" && interface.is_none();
                return named(line, line_start, role?, interface.map_or(after, str::trim_start), is_function);
            }
            _ if MODIFIERS.contains(&word) => {}
            _ => return None,
        }
        if after.is_empty() {
            return None;
        }
        rest = after;
    }
}

/// The declared name at the start of `after` (what follows the keyword),
/// skipping type parameters and a receiver: `<T> List<T>.first` names `first`.
fn named(line: &str, line_start: usize, role: Role, after: &str, is_function: bool) -> Option<Declaration> {
    let mut at = line.len() - after.len();
    let bytes = line.as_bytes();
    skip_angles(bytes, &mut at);
    skip_spaces(bytes, &mut at);
    let mut name = None;
    loop {
        let start = at;
        while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_') {
            at += 1;
        }
        if at == start {
            break;
        }
        name = Some((start, at));
        skip_angles(bytes, &mut at);
        if bytes.get(at) == Some(&b'?') {
            at += 1;
        }
        if bytes.get(at) == Some(&b'.') {
            at += 1;
        } else {
            break;
        }
    }
    let (start, end) = name?;
    let parameters = if is_function { parameter_types(&line[end..]) } else { None };
    Some(Declaration { role, name: line[start..end].to_string(), name_byte: line_start + start, parameters })
}

/// The parameter types of the list `text` starts with (`(a: Int, b: List<String>)`),
/// whitespace removed, or `None` when it is not one or does not close in `text`.
fn parameter_types(text: &str) -> Option<Vec<String>> {
    let inner = text.trim_start().strip_prefix('(')?;
    let mut depth = 0usize;
    let mut parts: Vec<&str> = Vec::new();
    let mut part_start = 0;
    let mut previous = ' ';
    for (at, c) in inner.char_indices() {
        match c {
            '(' | '<' | '[' => depth += 1,
            '>' if previous == '-' => {}
            ')' | '>' | ']' if depth > 0 => depth -= 1,
            ')' => {
                parts.push(&inner[part_start..at]);
                return Some(
                    parts
                        .into_iter()
                        .filter(|p| !p.trim().is_empty())
                        .map(|p| {
                            let ty = p.split_once(':').map_or("", |(_, ty)| ty);
                            ty.split_once('=').map_or(ty, |(ty, _)| ty).split_whitespace().collect()
                        })
                        .collect(),
                );
            }
            ',' if depth == 0 => {
                parts.push(&inner[part_start..at]);
                part_start = at + 1;
            }
            _ => {}
        }
        previous = c;
    }
    None
}

fn skip_spaces(bytes: &[u8], at: &mut usize) {
    while bytes.get(*at).is_some_and(u8::is_ascii_whitespace) {
        *at += 1;
    }
}

/// Moves `at` past a balanced `<...>` starting there, if there is one.
fn skip_angles(bytes: &[u8], at: &mut usize) {
    if bytes.get(*at) != Some(&b'<') {
        return;
    }
    let mut depth = 0;
    while let Some(&byte) = bytes.get(*at) {
        *at += 1;
        match byte {
            b'<' => depth += 1,
            b'>' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
    }
}

/// The declarations facing the one on the line containing `byte` in `file`:
/// every `actual` of an `expect` and every `expect` of an `actual`, across the
/// module's source sets. Empty for a file outside a multiplatform module and
/// for a line that declares neither.
pub fn counterparts(project_root: &Path, file: &Path, source: &str, byte: usize) -> Vec<Counterpart> {
    let Some((module, _)) = locate(project_root, file) else {
        return Vec::new();
    };
    let at = byte.min(source.len());
    let line_start = source[..at].rfind('\n').map_or(0, |i| i + 1);
    let line_end = source[at..].find('\n').map_or(source.len(), |i| at + i + 1);
    let Some(here) = declaration_in(&source[line_start..line_end], line_start) else {
        return Vec::new();
    };
    let wanted = match here.role {
        Role::Expect => Role::Actual,
        Role::Actual => Role::Expect,
    };

    let mut found: Vec<Counterpart> = module_declarations(&module)
        .into_iter()
        .filter(|(_, _, declaration)| declaration.role == wanted && declaration.pairs_with(&here))
        .map(|(label, path, declaration)| Counterpart { path, byte_offset: declaration.name_byte, label })
        .collect();
    found.sort_by(|a, b| (&a.label, &a.path, a.byte_offset).cmp(&(&b.label, &b.path, b.byte_offset)));
    found
}

/// The 0-based lines of `source` that declare an `expect` or an `actual` —
/// where the gutter marks a jump. Text only, no project scan.
pub fn declaration_lines(source: &str) -> Vec<usize> {
    if !source.contains("expect") && !source.contains("actual") {
        return Vec::new();
    }
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| declaration_in(line, 0).is_some())
        .map(|(index, _)| index)
        .collect()
}

/// The `expect` declarations the identifier at `byte` may refer to, for a call
/// site a language server could not resolve (it analyzes every source set as
/// one module and answers nothing for a call to an `expect`). Matched by name,
/// like [`counterparts`]; empty outside a multiplatform module or when the
/// identifier names no `expect`. The declaration under the caret itself is
/// left out, so a jump never lands where it started.
pub fn expect_definitions(project_root: &Path, file: &Path, source: &str, byte: usize) -> Vec<Counterpart> {
    let Some((module, _)) = locate(project_root, file) else {
        return Vec::new();
    };
    let Some(name) = identifier_at(source, byte) else {
        return Vec::new();
    };
    let mut found: Vec<Counterpart> = module_declarations(&module)
        .into_iter()
        .filter(|(_, path, d)| {
            d.role == Role::Expect && d.name == name && !(path == file && (d.name_byte..d.name_byte + name.len()).contains(&byte))
        })
        .map(|(label, path, d)| Counterpart { path, byte_offset: d.name_byte, label })
        .collect();
    found.sort_by(|a, b| (&a.label, &a.path, a.byte_offset).cmp(&(&b.label, &b.path, b.byte_offset)));
    found
}

/// The identifier (letters, digits, underscore) containing or ending at `byte`.
fn identifier_at(source: &str, byte: usize) -> Option<&str> {
    let bytes = source.as_bytes();
    let at = byte.min(bytes.len());
    let is_word = |b: &u8| b.is_ascii_alphanumeric() || *b == b'_';
    let start = bytes[..at].iter().rposition(|b| !is_word(b)).map_or(0, |i| i + 1);
    let end = at + bytes[at..].iter().position(|b| !is_word(b)).unwrap_or(bytes.len() - at);
    (start < end).then(|| &source[start..end])
}

/// Every `expect`/`actual` declaration in the module's sources, with the
/// source set and file each sits in.
fn module_declarations(module: &Path) -> Vec<(String, PathBuf, Declaration)> {
    let mut all = Vec::new();
    for set_dir in source_set_dirs(module) {
        let set = set_dir.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        for dir in SOURCE_DIRS.iter().filter(|d| **d != "resources") {
            for path in kotlin_files(&set_dir.join(dir)) {
                let Ok(text) = std::fs::read_to_string(&path) else { continue };
                all.extend(declarations(&text).into_iter().map(|d| (set.clone(), path.clone(), d)));
            }
        }
    }
    all
}

/// The source set's name without its `Main`/`Test` suffix (`jvm` for
/// `jvmMain`), and which of the two it was.
fn split_set(set: &str) -> Option<(&str, &str)> {
    ["Main", "Test"].iter().find_map(|suffix| set.strip_suffix(suffix).map(|base| (base, *suffix)))
}

/// A warning on every `expect` in `source` that some declared target gets no
/// `actual` for. Matched like [`counterparts`]: by name and, for a function whose
/// parameters sit on one line, by parameter types, so an `actual` for another
/// overload does not count. A target is served by an
/// `actual` in its own source set or in an intermediate one that covers it
/// (`iosMain` for `iosArm64`). A file with no `expect` costs one pass over its
/// own text; the module is only read when it has one.
///
/// Sets this does not understand (`androidMain` with no `androidTarget`
/// declared, say) are taken to serve every target rather than guessed at, and
/// a build file whose targets cannot be read falls back to "no `actual`
/// anywhere".
pub fn missing_actuals(project_root: &Path, file: &Path, source: &str) -> Vec<FileDiagnostic> {
    let expects: Vec<Declaration> = declarations(source).into_iter().filter(|d| d.role == Role::Expect).collect();
    if expects.is_empty() {
        return Vec::new();
    }
    let Some((module, own_set)) = locate(project_root, file) else {
        return Vec::new();
    };
    let declared = targets(&build_file_text(&module));
    let actuals: Vec<(String, Declaration)> = module_declarations(&module)
        .into_iter()
        .filter(|(_, _, d)| d.role == Role::Actual)
        .map(|(set, _, d)| (set, d))
        .collect();
    let (own_base, own_suffix) = split_set(&own_set).unwrap_or(("common", "Main"));

    expects
        .into_iter()
        .filter_map(|d| {
            let named: Vec<&str> =
                actuals.iter().filter(|(_, actual)| d.pairs_with(actual)).map(|(set, _)| set.as_str()).collect();
            let message = if named.is_empty() {
                format!("expect '{}' has no actual declaration", d.name)
            } else {
                let needed = covered(own_base, &declared);
                let unserved: Vec<&str> = needed
                    .iter()
                    .filter(|target| {
                        !named.iter().any(|set| match split_set(set) {
                            Some((base, suffix)) if suffix == own_suffix && base != "common" => {
                                let covers = covered(base, &declared);
                                covers.is_empty() || covers.contains(target)
                            }
                            // An `actual` in a set this cannot place.
                            _ => true,
                        })
                    })
                    .map(|target| target.name.as_str())
                    .collect();
                if unserved.is_empty() {
                    return None;
                }
                format!("expect '{}' has no actual for {}", d.name, unserved.join(", "))
            };
            Some(FileDiagnostic {
                range: d.name_byte..d.name_byte + d.name.len(),
                severity: ProblemSeverity::Warning,
                message,
            })
        })
        .collect()
}

fn source_set_dirs(module: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(module.join("src")) else {
        return Vec::new();
    };
    entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()
}

fn kotlin_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for path in entries.flatten().map(|e| e.path()) {
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "kt") {
                files.push(path);
            }
        }
    }
    files
}

#[cfg(test)]
#[path = "expect_actual_test.rs"]
mod expect_actual_test;
