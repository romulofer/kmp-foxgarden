use super::*;

fn names(source: &str) -> Vec<(Role, String)> {
    declarations(source).into_iter().map(|d| (d.role, d.name)).collect()
}

#[test]
fn reads_the_role_and_name_of_every_kind_of_declaration() {
    let source = "\
expect class Platform
expect fun platformName(): String
expect val cores: Int
expect interface Clock
expect object Registry
expect fun interface Handler
expect annotation class Marker
expect enum class Mode
expect fun <T> List<T>.firstOrDefault(default: T): T
expect fun String?.shout(): String
internal actual suspend inline fun fetch(): Unit = TODO()
@Suppress(\"X\") actual typealias Id = Long
";
    let found: Vec<String> = names(source).into_iter().map(|(_, name)| name).collect();
    assert_eq!(
        found,
        ["Platform", "platformName", "cores", "Clock", "Registry", "Handler", "Marker", "Mode", "firstOrDefault", "shout", "fetch", "Id"]
    );
    assert_eq!(names(source)[10].0, Role::Actual);
    assert_eq!(names(source)[0].0, Role::Expect);
}

#[test]
fn ignores_lines_that_merely_mention_the_words() {
    let source = "val expect = 1\n// expect fun a()\nfun actualValue() {}\nprintln(\"actual fun b()\")\nclass Actual\n";
    assert!(names(source).is_empty());
}

#[test]
fn the_name_offset_points_at_the_name() {
    let source = "package p\n\nexpect fun greet(): String\n";
    let declaration = &declarations(source)[0];
    assert_eq!(&source[declaration.name_byte..declaration.name_byte + 5], "greet");
}

fn module() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle.kts"), "plugins { kotlin(\"multiplatform\") }").unwrap();
    for (path, text) in [
        ("src/commonMain/kotlin/Platform.kt", "expect fun platformName(): String\nexpect class Clock\n"),
        ("src/jvmMain/kotlin/Platform.jvm.kt", "actual fun platformName(): String = \"jvm\"\nactual class Clock\n"),
        ("src/iosMain/kotlin/p/Platform.ios.kt", "actual fun platformName(): String = \"ios\"\n"),
        ("src/jvmMain/kotlin/Other.kt", "actual fun somethingElse() {}\n"),
    ] {
        let file = dir.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    dir
}

fn counterparts_at(dir: &tempfile::TempDir, relative: &str, needle: &str) -> Vec<Counterpart> {
    let file = dir.path().join(relative);
    let source = std::fs::read_to_string(&file).unwrap();
    let byte = source.find(needle).unwrap();
    counterparts(dir.path(), &file, &source, byte)
}

#[test]
fn an_expect_lists_every_actual_with_its_source_set() {
    let dir = module();
    let found = counterparts_at(&dir, "src/commonMain/kotlin/Platform.kt", "platformName");
    let labels: Vec<&str> = found.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["iosMain", "jvmMain"]);
    assert!(found[1].path.ends_with("src/jvmMain/kotlin/Platform.jvm.kt"));
    assert_eq!(found[1].byte_offset, "actual fun ".len());
}

#[test]
fn an_actual_leads_back_to_its_expect() {
    let dir = module();
    let found = counterparts_at(&dir, "src/jvmMain/kotlin/Platform.jvm.kt", "Clock");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].label, "commonMain");
    assert_eq!(found[0].byte_offset, "expect fun platformName(): String\nexpect class ".len());
}

#[test]
fn a_caret_anywhere_on_the_line_counts() {
    let dir = module();
    let found = counterparts_at(&dir, "src/commonMain/kotlin/Platform.kt", "String");
    assert_eq!(found.len(), 2);
}

#[test]
fn nothing_for_a_line_without_one_or_a_declaration_with_no_counterpart() {
    let dir = module();
    assert!(counterparts_at(&dir, "src/jvmMain/kotlin/Other.kt", "somethingElse").is_empty());
    let file = dir.path().join("src/commonMain/kotlin/Platform.kt");
    assert!(counterparts(dir.path(), &file, "val x = 1\n", 3).is_empty());
}

#[test]
fn files_outside_a_multiplatform_module_have_none() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle.kts"), "plugins { kotlin(\"jvm\") }").unwrap();
    let file = dir.path().join("src/main/kotlin/A.kt");
    assert!(counterparts(dir.path(), &file, "expect fun a()\n", 0).is_empty());
}

#[test]
fn an_expect_nothing_fulfils_is_flagged_on_its_name() {
    let dir = module();
    let file = dir.path().join("src/commonMain/kotlin/Extra.kt");
    let source = "expect fun platformName(): String\nexpect class Lonely\n";
    let found = missing_actuals(dir.path(), &file, source);
    assert_eq!(found.len(), 1, "platformName has actuals, Lonely has none");
    assert_eq!(&source[found[0].range.clone()], "Lonely");
    assert_eq!(found[0].severity, ProblemSeverity::Warning);
    assert_eq!(found[0].message, "expect 'Lonely' has no actual declaration");
}

#[test]
fn files_without_an_expect_or_outside_a_module_are_not_flagged() {
    let dir = module();
    let file = dir.path().join("src/jvmMain/kotlin/Platform.jvm.kt");
    assert!(missing_actuals(dir.path(), &file, "actual fun a() {}\n").is_empty());
    let plain = tempfile::tempdir().unwrap();
    std::fs::write(plain.path().join("build.gradle.kts"), "plugins { kotlin(\"jvm\") }").unwrap();
    let file = plain.path().join("src/main/kotlin/A.kt");
    assert!(missing_actuals(plain.path(), &file, "expect fun a()\n").is_empty());
}

#[test]
fn declaration_lines_are_the_expect_and_actual_ones() {
    let source = "package p\n\nexpect fun a(): Int\nfun plain() {}\nprivate actual class B\nval expected = 1\n";
    assert_eq!(declaration_lines(source), vec![2, 4]);
    assert!(declaration_lines("fun main() {}\n").is_empty());
}

fn targeted_module(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("build.gradle.kts"),
        "plugins { kotlin(\"multiplatform\") }\nkotlin {\n    jvm()\n    iosArm64()\n    iosX64()\n    linuxX64()\n}\n",
    )
    .unwrap();
    for (path, text) in files {
        let file = dir.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    dir
}

fn messages(dir: &tempfile::TempDir) -> Vec<String> {
    let file = dir.path().join("src/commonMain/kotlin/Platform.kt");
    let source = std::fs::read_to_string(&file).unwrap();
    missing_actuals(dir.path(), &file, &source).into_iter().map(|d| d.message).collect()
}

#[test]
fn an_expect_names_the_targets_that_lack_an_actual() {
    let dir = targeted_module(&[
        ("src/commonMain/kotlin/Platform.kt", "expect fun name(): String\n"),
        ("src/jvmMain/kotlin/Platform.jvm.kt", "actual fun name(): String = \"jvm\"\n"),
        ("src/iosArm64Main/kotlin/Platform.ios.kt", "actual fun name(): String = \"ios\"\n"),
    ]);
    assert_eq!(messages(&dir), vec!["expect 'name' has no actual for iosX64, linuxX64"]);
}

#[test]
fn an_actual_in_an_intermediate_set_serves_the_targets_it_groups() {
    let dir = targeted_module(&[
        ("src/commonMain/kotlin/Platform.kt", "expect fun name(): String\n"),
        ("src/jvmMain/kotlin/Platform.jvm.kt", "actual fun name(): String = \"jvm\"\n"),
        ("src/iosMain/kotlin/Platform.ios.kt", "actual fun name(): String = \"ios\"\n"),
        ("src/linuxX64Main/kotlin/Platform.linux.kt", "actual fun name(): String = \"linux\"\n"),
    ]);
    assert!(messages(&dir).is_empty());
}

#[test]
fn a_native_set_serves_every_native_target_but_not_the_jvm() {
    let dir = targeted_module(&[
        ("src/commonMain/kotlin/Platform.kt", "expect fun name(): String\n"),
        ("src/nativeMain/kotlin/Platform.native.kt", "actual fun name(): String = \"native\"\n"),
    ]);
    assert_eq!(messages(&dir), vec!["expect 'name' has no actual for jvm"]);
}

#[test]
fn an_actual_in_a_set_no_declared_target_matches_is_not_second_guessed() {
    let dir = targeted_module(&[
        ("src/commonMain/kotlin/Platform.kt", "expect fun name(): String\n"),
        ("src/androidMain/kotlin/Platform.android.kt", "actual fun name(): String = \"android\"\n"),
    ]);
    assert!(messages(&dir).is_empty());
}
