use super::*;

fn names(build_file: &str) -> Vec<String> {
    targets(build_file).into_iter().map(|t| t.name).collect()
}

#[test]
fn reads_the_targets_a_build_file_declares() {
    let build = "\
kotlin {
    jvm()
    js(IR) { browser() }
    wasmJs { browser() }
    iosArm64()
    iosX64 { binaries.framework() }
    linuxX64()
    // macosArm64()
    androidTarget()
    sourceSets { val commonMain by getting }
}
";
    assert_eq!(names(build), ["jvm", "js", "wasmJs", "iosArm64", "iosX64", "linuxX64"]);
}

#[test]
fn a_renamed_target_gives_its_tasks_the_new_name() {
    assert_eq!(names("kotlin { jvm(\"desktop\") }"), ["desktop"]);
    let found = targets("kotlin { jvm(\"desktop\") }");
    assert_eq!(task_names(&found), ["allTests", "desktopTest"]);
}

#[test]
fn words_that_only_contain_a_target_name_do_not_count() {
    assert!(names("val jvmTarget = 17\nconfigure(jvmToolchain(17))\nval x = iosVersion\nfoo.js = 1").is_empty());
}

#[test]
fn native_targets_also_offer_to_link_a_debug_executable() {
    let found = targets("kotlin { jvm(); iosArm64(); js() }");
    assert_eq!(
        task_names(&found),
        ["allTests", "jvmTest", "iosArm64Test", "linkDebugExecutableIosArm64", "jsTest"]
    );
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let write = |path: &str, text: &str| {
        let file = dir.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    };
    write("build.gradle.kts", "plugins { kotlin(\"jvm\") }\n");
    write("shared/build.gradle.kts", "plugins { kotlin(\"multiplatform\") }\nkotlin { jvm(); iosArm64() }\n");
    write("androidApp/build.gradle.kts", "plugins { kotlin(\"android\") }\n");
    write("build/shared/build.gradle.kts", "plugins { kotlin(\"multiplatform\") }\nkotlin { jvm() }\n");
    dir
}

#[test]
fn every_multiplatform_module_lists_its_tasks_with_its_gradle_path() {
    let dir = project();
    let labels: Vec<String> = project_tasks(dir.path()).into_iter().map(|t| t.label).collect();
    assert_eq!(
        labels,
        [
            "Gradle: :shared:allTests",
            "Gradle: :shared:jvmTest",
            "Gradle: :shared:iosArm64Test",
            "Gradle: :shared:linkDebugExecutableIosArm64",
        ]
    );
}

#[test]
fn the_root_module_needs_no_project_path_and_tasks_run_from_the_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle.kts"), "plugins { kotlin(\"multiplatform\") }\nkotlin { jvm() }\n").unwrap();
    let tasks = project_tasks(dir.path());
    assert_eq!(tasks[1].label, "Gradle: jvmTest");
    assert_eq!(tasks[1].command.args, ["--console=plain", "jvmTest"]);
    assert_eq!(tasks[1].command.cwd, dir.path());
}

#[test]
fn the_projects_wrapper_is_used_when_it_has_one() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle.kts"), "plugins { kotlin(\"multiplatform\") }\n").unwrap();
    let wrapper = dir.path().join(if cfg!(windows) { "gradlew.bat" } else { "gradlew" });
    assert_ne!(gradle_program(dir.path()), wrapper, "no wrapper yet");
    std::fs::write(&wrapper, "").unwrap();
    assert_eq!(gradle_program(dir.path()), wrapper);
}

#[test]
fn a_project_with_no_multiplatform_module_has_no_tasks() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle.kts"), "plugins { kotlin(\"jvm\") }\n").unwrap();
    assert!(project_tasks(dir.path()).is_empty());
}

#[test]
fn test_tasks_name_the_directories_they_report_into() {
    let dir = project();
    let tasks = project_tasks(dir.path());
    let reports = dir.path().join("shared/build/test-results");
    let by_label = |label: &str| tasks.iter().find(|t| t.label == label).unwrap();

    assert_eq!(by_label("Gradle: :shared:jvmTest").report_dirs, [reports.join("jvmTest")]);
    assert_eq!(
        by_label("Gradle: :shared:allTests").report_dirs,
        [reports.join("jvmTest"), reports.join("iosArm64Test")]
    );
    assert!(by_label("Gradle: :shared:linkDebugExecutableIosArm64").report_dirs.is_empty());
}

fn module_with(targets: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("build.gradle.kts"),
        format!("plugins {{ kotlin(\"multiplatform\") }}\nkotlin {{ {targets} }}\n"),
    )
    .unwrap();
    dir
}

fn file_task_args(dir: &tempfile::TempDir, relative: &str) -> Option<Vec<String>> {
    file_task(dir.path(), &dir.path().join(relative)).map(|t| t.command.args[1..].to_vec())
}

#[test]
fn common_code_runs_every_targets_tests() {
    let dir = module_with("jvm(); iosArm64()");
    assert_eq!(file_task_args(&dir, "src/commonMain/kotlin/A.kt").unwrap(), ["allTests"]);
    assert_eq!(file_task_args(&dir, "src/commonTest/kotlin/A.kt").unwrap(), ["allTests"]);
}

#[test]
fn a_platform_file_runs_that_platforms_tests() {
    let dir = module_with("jvm(); iosArm64(); iosX64()");
    assert_eq!(file_task_args(&dir, "src/jvmMain/kotlin/A.kt").unwrap(), ["jvmTest"]);
    assert_eq!(file_task_args(&dir, "src/jvmTest/kotlin/A.kt").unwrap(), ["jvmTest"]);
}

#[test]
fn an_intermediate_source_set_runs_the_tests_of_the_targets_it_groups() {
    let dir = module_with("jvm(); iosArm64(); iosX64(); linuxX64()");
    assert_eq!(file_task_args(&dir, "src/iosMain/kotlin/A.kt").unwrap(), ["iosArm64Test", "iosX64Test"]);
    assert_eq!(file_task_args(&dir, "src/nativeMain/kotlin/A.kt").unwrap(), ["iosArm64Test", "iosX64Test", "linuxX64Test"]);
    assert_eq!(file_task_args(&dir, "src/appleMain/kotlin/A.kt").unwrap(), ["iosArm64Test", "iosX64Test"]);
}

#[test]
fn no_task_for_a_source_set_no_target_covers_or_a_file_outside_a_module() {
    let dir = module_with("jvm()");
    assert_eq!(file_task_args(&dir, "src/iosMain/kotlin/A.kt"), None);
    assert_eq!(file_task_args(&dir, "src/jvmMain/resources/a.txt").unwrap(), ["jvmTest"]);
    assert_eq!(file_task_args(&dir, "README.md"), None);
}

#[test]
fn a_submodule_file_gets_a_task_with_its_project_path() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("shared")).unwrap();
    std::fs::write(
        dir.path().join("shared/build.gradle.kts"),
        "plugins { kotlin(\"multiplatform\") }\nkotlin { jvm() }\n",
    )
    .unwrap();
    let task = file_task(dir.path(), &dir.path().join("shared/src/jvmTest/kotlin/A.kt")).unwrap();
    assert_eq!(task.label, "Gradle: :shared:jvmTest");
    assert_eq!(task.report_dirs, [dir.path().join("shared/build/test-results/jvmTest")]);
}

#[test]
fn a_js_target_with_a_browser_offers_its_development_server() {
    let found = targets("kotlin {\n    js(IR) {\n        browser {\n        }\n    }\n    wasmJs { nodejs() }\n    jvm()\n}\n");
    assert_eq!(
        task_names(&found),
        ["allTests", "jsTest", "jsBrowserDevelopmentRun", "wasmJsTest", "jvmTest"]
    );
}

#[test]
fn a_browser_call_outside_a_js_target_is_not_attributed_to_it() {
    let found = targets("kotlin {\n    js()\n    jvm()\n    browser()\n}\n");
    assert_eq!(task_names(&found), ["allTests", "jsTest", "jvmTest"]);
}
