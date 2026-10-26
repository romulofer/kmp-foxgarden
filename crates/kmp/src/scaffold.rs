//! The new-project template for a Kotlin Multiplatform module (`PLAN.md`
//! Track 30 Phase 1).
//!
//! Pure text generation, like the JVM scaffolds: a Gradle build applying the
//! multiplatform plugin with one `jvm()` target (the only one that builds
//! with nothing but a JDK), an `expect`/`actual` pair, a `main` the JVM run
//! path can launch and a common test. More targets are one line each in the
//! build file.

use std::path::{Path, PathBuf};

use fg_extension::{ScaffoldContribution, ScaffoldSpec};

const GRADLE: &str = "gradle";
const KOTLIN: &str = "kotlin";

/// The template's id in the wizard and in [`ScaffoldSpec::template_id`].
pub(crate) const TEMPLATE: &str = "multiplatform";

/// The Kotlin release the generated build pins: a resolvable plugin version,
/// so the project builds without the user picking one. 2.3 is the first this
/// was verified with on a current JDK (2.0.21's compiler fails on JDK 26).
const KOTLIN_VERSION: &str = "2.3.0";

const GITIGNORE: &str = "build/\n.gradle/\n.kotlin/\n.idea/\n*.iml\n";

pub fn contributions() -> Vec<ScaffoldContribution> {
    vec![ScaffoldContribution {
        build_tool_id: GRADLE.to_string(),
        language_id: KOTLIN.to_string(),
        template_id: TEMPLATE.to_string(),
        template_name: "Kotlin Multiplatform".to_string(),
        runtime_versions: Vec::new(),
    }]
}

/// The files of a new multiplatform project, or `None` for any other kind.
pub fn scaffold_files(spec: &ScaffoldSpec) -> Option<Vec<(PathBuf, String)>> {
    if spec.template_id != TEMPLATE || spec.build_tool_id != GRADLE || spec.language_id != KOTLIN {
        return None;
    }
    let segments: Vec<&str> = spec.namespace.split('.').filter(|s| !s.is_empty()).collect();
    let package = segments.join(".");
    let package_dir: PathBuf = segments.iter().collect();
    let header = if package.is_empty() { String::new() } else { format!("package {package}\n\n") };
    let source = |set: &str, file: &str| Path::new("src").join(set).join("kotlin").join(&package_dir).join(file);

    Some(vec![
        (PathBuf::from("settings.gradle.kts"), format!("rootProject.name = \"{}\"\n", spec.name)),
        (PathBuf::from("build.gradle.kts"), build_gradle_kts()),
        (
            source("commonMain", "Platform.kt"),
            format!(
                "{header}expect fun platformName(): String\n\nfun greeting(): String = \"Hello from ${{platformName()}}!\"\n"
            ),
        ),
        (source("jvmMain", "Platform.jvm.kt"), format!("{header}actual fun platformName(): String = \"JVM\"\n")),
        (source("jvmMain", "Main.kt"), format!("{header}fun main() {{\n    println(greeting())\n}}\n")),
        (
            source("commonTest", "GreetingTest.kt"),
            format!(
                "{header}import kotlin.test.Test\nimport kotlin.test.assertTrue\n\nclass GreetingTest {{\n    @Test\n    fun greetingNamesThePlatform() {{\n        assertTrue(greeting().contains(platformName()))\n    }}\n}}\n"
            ),
        ),
        (PathBuf::from(".gitignore"), GITIGNORE.to_string()),
    ])
}

fn build_gradle_kts() -> String {
    format!(
        r#"plugins {{
    kotlin("multiplatform") version "{KOTLIN_VERSION}"
}}

repositories {{
    mavenCentral()
}}

kotlin {{
    // More targets: js(IR) {{ browser() }}, linuxX64(), iosArm64(), ...
    jvm()

    sourceSets {{
        commonTest.dependencies {{
            implementation(kotlin("test"))
        }}
    }}
}}
"#
    )
}

#[cfg(test)]
#[path = "scaffold_test.rs"]
mod scaffold_test;
