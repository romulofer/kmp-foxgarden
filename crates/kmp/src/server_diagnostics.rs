//! Language server diagnostics that are wrong in a multiplatform module
//! (`PLAN.md` Track 30 Phase 4).
//!
//! `kotlin-language-server` analyzes every source set of a module as one
//! compilation, so each `actual` collides with the others and with its
//! `expect`: measured against a module with `jvm()` and `linuxX64()`, it
//! reported `CONFLICTING_OVERLOADS`, `REDECLARATION`,
//! `PACKAGE_OR_CLASSIFIER_REDECLARATION`, `OVERLOAD_RESOLUTION_AMBIGUITY`,
//! `AMBIGUOUS_ACTUALS` and `EXPECT_AND_ACTUAL_IN_THE_SAME_MODULE` on every
//! declaration of an `expect`/`actual` pair, which the real compiler accepts.
//! Genuine type errors came through correctly, so only these are dropped.

use std::path::Path;

use crate::source_set::locate;

/// Reported on an `expect` and its `actual`s only because they share one
/// compilation.
const ALWAYS_SPURIOUS: [&str; 2] = ["AMBIGUOUS_ACTUALS", "EXPECT_AND_ACTUAL_IN_THE_SAME_MODULE"];

/// Reported on any duplicate declaration. Spurious when the message names
/// declarations in more than one source set, since the compiler never sees
/// those together; a duplicate within one source set is a real error.
const REDECLARATIONS: [&str; 4] =
    ["CONFLICTING_OVERLOADS", "REDECLARATION", "PACKAGE_OR_CLASSIFIER_REDECLARATION", "OVERLOAD_RESOLUTION_AMBIGUITY"];

/// Whether the server's diagnostic (`code`, `message`) on `file` is one of the
/// false positives above. Returns without touching the disk for any other code.
pub fn is_spurious(project_root: &Path, file: &Path, code: &str, message: &str) -> bool {
    let always = ALWAYS_SPURIOUS.contains(&code);
    if !always && !REDECLARATIONS.contains(&code) {
        return false;
    }
    let Some((_, own_set)) = locate(project_root, file) else {
        return false;
    };
    always || source_sets_named(message, &own_set).len() > 1
}

/// The distinct source sets whose files `message` names (`.../src/jvmMain/...`),
/// together with `own_set` when the message names nothing at all.
fn source_sets_named(message: &str, own_set: &str) -> Vec<String> {
    let mut sets: Vec<String> = Vec::new();
    let mut rest = message;
    while let Some(at) = rest.find("/src/") {
        rest = &rest[at + "/src/".len()..];
        let set = rest.split('/').next().unwrap_or_default();
        if !set.is_empty() && !sets.iter().any(|known| known == set) {
            sets.push(set.to_string());
        }
    }
    if sets.is_empty() {
        sets.push(own_set.to_string());
    }
    sets
}

#[cfg(test)]
#[path = "server_diagnostics_test.rs"]
mod server_diagnostics_test;
