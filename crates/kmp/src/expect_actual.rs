//! `expect`/`actual` navigation (`PLAN.md` Track 30 Phase 2).
//!
//! A multiplatform module declares an API once with `expect` in common code
//! and fulfils it with an `actual` in each platform's source set. Both
//! modifiers are syntactic, so declarations are found by reading a line's
//! modifiers and name rather than by running a compiler, and matched by
//! name: an overloaded function lists every overload, and the picker lets
//! the user choose. The scan reads every source file of the module, which
//! is why it answers a command and never a frame.

use std::path::{Path, PathBuf};

use fg_extension::Counterpart;

use crate::source_set::{SOURCE_DIRS, locate};

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
            "typealias" => return named(line, line_start, role?, after),
            _ if KEYWORDS.contains(&word) && role.is_some() => {
                // `fun interface Name`: the keyword is `interface`.
                let after = if word == "fun" { after.strip_prefix("interface").map_or(after, str::trim_start) } else { after };
                return named(line, line_start, role?, after);
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
fn named(line: &str, line_start: usize, role: Role, after: &str) -> Option<Declaration> {
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
    Some(Declaration { role, name: line[start..end].to_string(), name_byte: line_start + start })
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

    let mut found = Vec::new();
    for set_dir in source_set_dirs(&module) {
        let set = set_dir.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        for dir in SOURCE_DIRS.iter().filter(|d| **d != "resources") {
            for path in kotlin_files(&set_dir.join(dir)) {
                let Ok(text) = std::fs::read_to_string(&path) else { continue };
                for declaration in declarations(&text) {
                    if declaration.role == wanted && declaration.name == here.name {
                        found.push(Counterpart { path: path.clone(), byte_offset: declaration.name_byte, label: set.clone() });
                    }
                }
            }
        }
    }
    found.sort_by(|a, b| (&a.label, &a.path, a.byte_offset).cmp(&(&b.label, &b.path, b.byte_offset)));
    found
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
