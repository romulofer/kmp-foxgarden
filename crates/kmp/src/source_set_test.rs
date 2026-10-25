use super::*;

fn project(build_file: &str, text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(build_file), text).unwrap();
    dir
}

fn source_set_of(dir: &tempfile::TempDir, relative: &str) -> Option<String> {
    source_set(dir.path(), &dir.path().join(relative))
}

#[test]
fn files_in_a_multiplatform_module_report_their_source_set() {
    let dir = project("build.gradle.kts", "plugins {\n    kotlin(\"multiplatform\") version \"2.0.0\"\n}\n");
    assert_eq!(source_set_of(&dir, "src/commonMain/kotlin/app/Greeter.kt").as_deref(), Some("commonMain"));
    assert_eq!(source_set_of(&dir, "src/jvmTest/kotlin/GreeterTest.kt").as_deref(), Some("jvmTest"));
    assert_eq!(source_set_of(&dir, "src/iosMain/kotlin/Platform.ios.kt").as_deref(), Some("iosMain"));
    assert_eq!(source_set_of(&dir, "src/commonMain/resources/strings.properties").as_deref(), Some("commonMain"));
}

#[test]
fn every_spelling_of_the_plugin_is_recognized() {
    for build in [
        ("build.gradle.kts", "plugins { id(\"org.jetbrains.kotlin.multiplatform\") }"),
        ("build.gradle.kts", "plugins { alias(libs.plugins.kotlin.multiplatform) }"),
        ("build.gradle", "plugins { id 'org.jetbrains.kotlin.multiplatform' version '2.0.0' }"),
        ("build.gradle", "apply plugin: 'kotlin-multiplatform'"),
    ] {
        let dir = project(build.0, build.1);
        assert!(source_set_of(&dir, "src/commonMain/kotlin/A.kt").is_some(), "{build:?}");
    }
}

#[test]
fn an_ordinary_module_has_no_source_sets() {
    let dir = project("build.gradle.kts", "plugins { kotlin(\"jvm\") version \"2.0.0\" }");
    assert_eq!(source_set_of(&dir, "src/main/kotlin/A.kt"), None);
    assert_eq!(source_set_of(&dir, "src/commonMain/kotlin/A.kt"), None, "the plugin, not the directory name, decides");
}

#[test]
fn a_commented_out_plugin_does_not_count() {
    let dir = project("build.gradle.kts", "plugins {\n    // kotlin(\"multiplatform\")\n    kotlin(\"jvm\")\n}");
    assert_eq!(source_set_of(&dir, "src/commonMain/kotlin/A.kt"), None);
}

#[test]
fn the_module_is_the_directory_that_holds_src_not_the_project_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("shared")).unwrap();
    std::fs::create_dir_all(dir.path().join("androidApp")).unwrap();
    std::fs::write(dir.path().join("shared/build.gradle.kts"), "plugins { kotlin(\"multiplatform\") }").unwrap();
    std::fs::write(dir.path().join("androidApp/build.gradle.kts"), "plugins { kotlin(\"android\") }").unwrap();
    assert_eq!(
        source_set(dir.path(), &dir.path().join("shared/src/commonMain/kotlin/A.kt")).as_deref(),
        Some("commonMain")
    );
    assert_eq!(source_set(dir.path(), &dir.path().join("androidApp/src/main/kotlin/A.kt")), None);
}

#[test]
fn files_outside_the_source_layout_or_the_project_have_none() {
    let dir = project("build.gradle.kts", "plugins { kotlin(\"multiplatform\") }");
    assert_eq!(source_set_of(&dir, "build.gradle.kts"), None);
    assert_eq!(source_set_of(&dir, "src/commonMain/Stray.kt"), None, "needs a kotlin/java/resources directory");
    assert_eq!(source_set(dir.path(), Path::new("/elsewhere/src/commonMain/kotlin/A.kt")), None);
}

#[test]
fn an_intermediate_set_names_the_declared_targets_it_groups() {
    let dir = project(
        "build.gradle.kts",
        "plugins { kotlin(\"multiplatform\") }\nkotlin {\n    jvm()\n    iosArm64()\n    iosX64()\n    linuxX64()\n}\n",
    );
    assert_eq!(source_set_of(&dir, "src/iosMain/kotlin/A.kt").as_deref(), Some("iosMain (iosArm64, iosX64)"));
    assert_eq!(source_set_of(&dir, "src/nativeTest/kotlin/A.kt").as_deref(), Some("nativeTest (iosArm64, iosX64, linuxX64)"));
    assert_eq!(source_set_of(&dir, "src/commonMain/kotlin/A.kt").as_deref(), Some("commonMain"));
    assert_eq!(source_set_of(&dir, "src/jvmMain/kotlin/A.kt").as_deref(), Some("jvmMain"));
    assert_eq!(source_set_of(&dir, "src/linuxX64Main/kotlin/A.kt").as_deref(), Some("linuxX64Main"));
}
