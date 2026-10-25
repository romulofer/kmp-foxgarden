//! Per-target Gradle tasks (`PLAN.md` Track 30 Phase 3).
//!
//! A multiplatform module declares its targets in the build file
//! (`jvm()`, `js(IR)`, `iosArm64()`), and Gradle derives a task set from each:
//! `jvmTest`, `linkDebugExecutableIosArm64`, plus the module-wide `allTests`.
//! The tasks are read from the build file's text, not from a Gradle run, so
//! listing them is cheap enough to do when the command palette opens. They
//! run through the project's own Gradle wrapper.

use std::path::{Path, PathBuf};

use fg_extension::{CommandSpec, ProjectTask};

use crate::source_set::uses_multiplatform_plugin;

/// Directories a module is not looked for in.
const SKIPPED: [&str; 7] = [".git", ".gradle", ".idea", "build", "node_modules", "out", "target"];

/// How far below the project root a module is looked for: the root itself
/// and one level of subprojects (`shared`, `composeApp`), then one more for
/// `libs/core`-style layouts.
const MAX_DEPTH: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Jvm,
    Js,
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    name: String,
    kind: Kind,
}

const NATIVE_PREFIXES: [&str; 6] = ["ios", "macos", "tvos", "watchos", "linux", "mingw"];

/// The targets a build file declares, in declaration order.
fn targets(build_file: &str) -> Vec<Target> {
    let mut found: Vec<Target> = Vec::new();
    for line in build_file.lines().map(str::trim).filter(|l| !l.starts_with("//")) {
        let bytes = line.as_bytes();
        let mut at = 0;
        while at < bytes.len() {
            if !(bytes[at].is_ascii_alphabetic()) || (at > 0 && is_word(bytes[at - 1])) {
                at += 1;
                continue;
            }
            let end = (at..bytes.len()).find(|&i| !is_word(bytes[i])).unwrap_or(bytes.len());
            let word = &line[at..end];
            let rest = line[end..].trim_start();
            at = end;
            let Some(kind) = kind_of(word) else { continue };
            if !(rest.starts_with('(') || rest.starts_with('{')) {
                continue;
            }
            // `jvm("desktop")` renames the target, and with it its tasks.
            let name = rest
                .strip_prefix('(')
                .and_then(|args| args.strip_prefix('"'))
                .and_then(|args| args.split('"').next())
                .filter(|name| !name.is_empty() && name.bytes().all(is_word))
                .unwrap_or(word);
            if !found.iter().any(|t| t.name == name) {
                found.push(Target { name: name.to_string(), kind });
            }
        }
    }
    found
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn kind_of(word: &str) -> Option<Kind> {
    match word {
        "jvm" => Some(Kind::Jvm),
        "js" | "wasmJs" => Some(Kind::Js),
        _ => NATIVE_PREFIXES
            .iter()
            .any(|prefix| word.strip_prefix(prefix).is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase())))
            .then_some(Kind::Native),
    }
}

fn capitalized(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
}

/// The task names a module with `targets` offers, `allTests` first.
fn task_names(targets: &[Target]) -> Vec<String> {
    let mut names = vec!["allTests".to_string()];
    for target in targets {
        names.push(format!("{}Test", target.name));
        if target.kind == Kind::Native {
            names.push(format!("linkDebugExecutable{}", capitalized(&target.name)));
        }
    }
    names
}

/// Multiplatform modules at or below `root`: the directory and its Gradle
/// project path (`""` for the root, `":shared"` below it).
fn modules(root: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut pending = vec![(root.to_path_buf(), String::new(), 0)];
    while let Some((dir, path, depth)) = pending.pop() {
        if uses_multiplatform_plugin(&dir) {
            found.push((dir.clone(), path.clone()));
        }
        if depth == MAX_DEPTH {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for child in entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            let Some(name) = child.file_name().and_then(|n| n.to_str()) else { continue };
            if name.starts_with('.') || SKIPPED.contains(&name) {
                continue;
            }
            pending.push((child.clone(), format!("{path}:{name}"), depth + 1));
        }
    }
    found.sort_by(|a, b| a.1.cmp(&b.1));
    found
}

/// The Gradle launcher for a project: its wrapper when it has one, else
/// whatever `gradle` is on the path.
fn gradle_program(root: &Path) -> PathBuf {
    let wrapper = root.join(if cfg!(windows) { "gradlew.bat" } else { "gradlew" });
    if wrapper.is_file() { wrapper } else { PathBuf::from(if cfg!(windows) { "gradle.bat" } else { "gradle" }) }
}

/// Every per-target task of every multiplatform module in the project.
pub fn project_tasks(root: &Path) -> Vec<ProjectTask> {
    let mut tasks = Vec::new();
    for (dir, path) in modules(root) {
        let text = ["build.gradle.kts", "build.gradle"]
            .iter()
            .find_map(|name| std::fs::read_to_string(dir.join(name)).ok())
            .unwrap_or_default();
        for name in task_names(&targets(&text)) {
            let task = format!("{path}:{name}");
            // The root module's tasks need no project path.
            let task = if path.is_empty() { name } else { task };
            let mut command = CommandSpec::new(gradle_program(root), root);
            command.args = vec!["--console=plain".to_string(), task.clone()];
            tasks.push(ProjectTask { label: format!("Gradle: {task}"), command });
        }
    }
    tasks
}

#[cfg(test)]
#[path = "tasks_test.rs"]
mod tasks_test;
