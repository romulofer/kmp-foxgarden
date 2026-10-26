use super::*;
use crate::source_set::source_set;
use crate::tasks::project_tasks;

fn spec() -> ScaffoldSpec {
    ScaffoldSpec {
        build_tool_id: GRADLE.to_string(),
        language_id: KOTLIN.to_string(),
        template_id: TEMPLATE.to_string(),
        namespace: "com.example.app".to_string(),
        name: "shared-lib".to_string(),
        runtime_version: None,
    }
}

fn paths(files: &[(PathBuf, String)]) -> Vec<String> {
    files.iter().map(|(p, _)| p.display().to_string().replace('\\', "/")).collect()
}

#[test]
fn the_template_is_a_gradle_kotlin_variant_with_no_runtime_to_pick() {
    let all = contributions();
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].build_tool_id.as_str(), all[0].language_id.as_str()), ("gradle", "kotlin"));
    assert_eq!(all[0].template_id, TEMPLATE);
    assert!(all[0].runtime_versions.is_empty());
}

#[test]
fn only_its_own_template_is_scaffolded() {
    assert!(scaffold_files(&ScaffoldSpec { template_id: String::new(), ..spec() }).is_none());
    assert!(scaffold_files(&ScaffoldSpec { build_tool_id: "maven".to_string(), ..spec() }).is_none());
    assert!(scaffold_files(&ScaffoldSpec { language_id: "java".to_string(), ..spec() }).is_none());
}

#[test]
fn sources_sit_in_source_sets_under_the_package() {
    let files = scaffold_files(&spec()).unwrap();
    assert_eq!(
        paths(&files),
        [
            "settings.gradle.kts",
            "build.gradle.kts",
            "src/commonMain/kotlin/com/example/app/Platform.kt",
            "src/jvmMain/kotlin/com/example/app/Platform.jvm.kt",
            "src/jvmMain/kotlin/com/example/app/Main.kt",
            "src/commonTest/kotlin/com/example/app/GreetingTest.kt",
            ".gitignore",
        ]
    );
    assert_eq!(files[0].1, "rootProject.name = \"shared-lib\"\n");
    assert!(files[2].1.starts_with("package com.example.app\n\nexpect fun platformName()"), "{}", files[2].1);
    assert!(files[3].1.contains("actual fun platformName()"));
}

#[test]
fn an_empty_namespace_writes_no_package_line_or_directory() {
    let files = scaffold_files(&ScaffoldSpec { namespace: String::new(), ..spec() }).unwrap();
    assert_eq!(paths(&files)[2], "src/commonMain/kotlin/Platform.kt");
    assert!(!files[2].1.contains("package"));
}

#[test]
fn the_generated_project_is_recognized_as_multiplatform_with_its_expect_actual_pair() {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in scaffold_files(&spec()).unwrap() {
        let file = dir.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    let common = dir.path().join("src/commonMain/kotlin/com/example/app/Platform.kt");
    assert_eq!(source_set(dir.path(), &common).as_deref(), Some("commonMain"));
    let labels: Vec<String> = project_tasks(dir.path()).into_iter().map(|t| t.label).collect();
    assert_eq!(labels, ["Gradle: allTests", "Gradle: jvmTest"]);
    let source = std::fs::read_to_string(&common).unwrap();
    assert!(crate::expect_actual::missing_actuals(dir.path(), &common, &source).is_empty());
}
