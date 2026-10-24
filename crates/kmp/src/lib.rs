//! Kotlin Multiplatform support for FoxGarden, as an extension (`PLAN.md`
//! Track 30).
//!
//! It contributes no language: Kotlin's grammar, language server and Gradle
//! tooling belong to the Java/Kotlin extension, and this one only adds what
//! exists in a multiplatform project. Phase 1 is source-set awareness; later
//! phases add `expect`/`actual` navigation and per-target tasks.

use std::path::Path;

use fg_extension::{CURRENT_SCHEMA_VERSION, Contributions, Extension, ExtensionManifest};

mod source_set;

pub struct KmpExtension;

impl Extension for KmpExtension {
    fn manifest(&self) -> ExtensionManifest {
        ExtensionManifest {
            id: "kmp".to_string(),
            name: "Kotlin Multiplatform".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: CURRENT_SCHEMA_VERSION,
        }
    }

    fn contributions(&self) -> Contributions {
        Contributions::default()
    }

    fn source_set(&self, project_root: &Path, file: &Path) -> Option<String> {
        source_set::source_set(project_root, file)
    }
}

#[cfg(test)]
#[path = "lib_test.rs"]
mod lib_test;
