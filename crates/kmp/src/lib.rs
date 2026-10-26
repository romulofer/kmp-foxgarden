//! Kotlin Multiplatform support for FoxGarden, as an extension (`PLAN.md`
//! Track 30).
//!
//! It contributes no language: Kotlin's grammar, language server and Gradle
//! tooling belong to the Java/Kotlin extension, and this one only adds what
//! exists in a multiplatform project: source-set awareness, `expect`/`actual`
//! navigation, per-target Gradle tasks and a new-project template.

use std::path::Path;

use fg_extension::{
    CURRENT_SCHEMA_VERSION, Contributions, Counterpart, Extension, ExtensionManifest, FileDiagnostic, ProjectTask, ScaffoldSpec,
    TaskTestResult,
};

mod expect_actual;
mod reports;
mod scaffold;
mod server_diagnostics;
mod source_set;
mod tasks;

/// Whether the new-project template is offered is the host's call: it is
/// built on the Gradle and Kotlin support of the JVM extension, so a host that
/// runs without that one registers this with [`KmpExtension::without_scaffold`].
pub struct KmpExtension {
    scaffold: bool,
}

impl KmpExtension {
    pub fn new() -> Self {
        Self { scaffold: true }
    }

    /// Everything but the new-project template, for a host without the
    /// Gradle build tool and Kotlin language it names.
    pub fn without_scaffold() -> Self {
        Self { scaffold: false }
    }
}

impl Default for KmpExtension {
    fn default() -> Self {
        Self::new()
    }
}

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
        Contributions { scaffolds: if self.scaffold { scaffold::contributions() } else { Vec::new() }, ..Contributions::default() }
    }

    fn scaffold_files(&self, spec: &ScaffoldSpec) -> Option<Vec<(std::path::PathBuf, String)>> {
        if self.scaffold { scaffold::scaffold_files(spec) } else { None }
    }

    fn source_set(&self, project_root: &Path, file: &Path) -> Option<String> {
        source_set::source_set(project_root, file)
    }

    fn task_test_results(&self, report_dirs: &[std::path::PathBuf]) -> Vec<TaskTestResult> {
        reports::task_test_results(report_dirs)
    }

    fn file_task(&self, project_root: &Path, file: &Path) -> Option<ProjectTask> {
        tasks::file_task(project_root, file)
    }

    fn project_tasks(&self, project_root: &Path) -> Vec<ProjectTask> {
        tasks::project_tasks(project_root)
    }

    fn file_diagnostics(&self, project_root: &Path, file: &Path, source: &str) -> Vec<FileDiagnostic> {
        expect_actual::missing_actuals(project_root, file, source)
    }

    fn counterparts(&self, project_root: &Path, file: &Path, source: &str, byte: usize) -> Vec<Counterpart> {
        expect_actual::counterparts(project_root, file, source, byte)
    }

    fn is_spurious_server_diagnostic(&self, project_root: &Path, file: &Path, code: &str, message: &str) -> bool {
        server_diagnostics::is_spurious(project_root, file, code, message)
    }

    fn definitions(&self, project_root: &Path, file: &Path, source: &str, byte: usize) -> Vec<Counterpart> {
        expect_actual::expect_definitions(project_root, file, source, byte)
    }

    fn counterpart_lines(&self, source: &str) -> Vec<usize> {
        expect_actual::declaration_lines(source)
    }
}

#[cfg(test)]
#[path = "lib_test.rs"]
mod lib_test;
