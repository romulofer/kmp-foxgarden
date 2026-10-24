//! Kotlin Multiplatform source sets (`PLAN.md` Track 30 Phase 1).
//!
//! A Kotlin Multiplatform module is a Gradle module applying the
//! `multiplatform` plugin, whose sources sit in one directory per source set
//! (`src/commonMain/kotlin`, `src/jvmTest/kotlin`, ...). Telling a file's
//! source set therefore takes only its path and one look at the module's
//! build file: no Gradle run, so it is cheap enough to ask whenever a tab is
//! focused.

use std::path::Path;

/// The directories under `src/<source set>/` that hold sources.
const SOURCE_DIRS: [&str; 3] = ["kotlin", "java", "resources"];

/// The source set `file` belongs to, when it sits in a Kotlin Multiplatform
/// module of the project at `project_root`. `None` for any other file,
/// including every file of an ordinary Gradle or Maven module, whose
/// `src/main` and `src/test` are not source sets in this sense.
pub fn source_set(project_root: &Path, file: &Path) -> Option<String> {
    let relative = file.strip_prefix(project_root).ok()?;
    let parts: Vec<&str> = relative.iter().filter_map(|p| p.to_str()).collect();
    // `src/<set>/<kotlin|java|resources>/<file...>`: the first `src` that
    // has that shape under it marks the module.
    let at = parts
        .windows(4)
        .position(|w| w[0] == "src" && SOURCE_DIRS.contains(&w[2]))?;
    let module = project_root.join(parts[..at].iter().collect::<std::path::PathBuf>());
    uses_multiplatform_plugin(&module).then(|| parts[at + 1].to_string())
}

/// Whether the Gradle build file in `module` applies the multiplatform
/// plugin, in any spelling a build uses: `kotlin("multiplatform")`,
/// `id("org.jetbrains.kotlin.multiplatform")`, a version-catalog alias such as
/// `libs.plugins.kotlin.multiplatform`, or `apply plugin: '...multiplatform'`.
fn uses_multiplatform_plugin(module: &Path) -> bool {
    ["build.gradle.kts", "build.gradle"].iter().any(|name| {
        std::fs::read_to_string(module.join(name)).is_ok_and(|text| {
            text.lines()
                .map(str::trim)
                .any(|line| !line.starts_with("//") && line.contains("multiplatform"))
        })
    })
}

#[cfg(test)]
#[path = "source_set_test.rs"]
mod source_set_test;
