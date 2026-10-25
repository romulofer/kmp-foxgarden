use super::*;

fn module() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle.kts"), "plugins { kotlin(\"multiplatform\") }").unwrap();
    dir
}

const ACROSS_SETS: &str = "Conflicting overloads: public actual fun platform(): String defined in root package in file /p/src/linuxX64Main/kotlin/Platform.linux.kt, public actual fun platform(): String defined in root package in file /p/src/jvmMain/kotlin/Platform.jvm.kt";
const WITHIN_A_SET: &str = "Conflicting overloads: public fun a(): Int defined in root package in file /p/src/jvmMain/kotlin/A.kt, public fun a(): Int defined in root package in file /p/src/jvmMain/kotlin/B.kt";

#[test]
fn a_redeclaration_across_source_sets_is_spurious() {
    let dir = module();
    let file = dir.path().join("src/jvmMain/kotlin/Platform.jvm.kt");
    for code in REDECLARATIONS {
        assert!(is_spurious(dir.path(), &file, code, ACROSS_SETS), "{code}");
    }
}

#[test]
fn a_redeclaration_within_one_source_set_is_real() {
    let dir = module();
    let file = dir.path().join("src/jvmMain/kotlin/A.kt");
    assert!(!is_spurious(dir.path(), &file, "CONFLICTING_OVERLOADS", WITHIN_A_SET));
}

#[test]
fn the_expect_actual_module_diagnostics_are_always_spurious() {
    let dir = module();
    let file = dir.path().join("src/commonMain/kotlin/Platform.kt");
    assert!(is_spurious(dir.path(), &file, "AMBIGUOUS_ACTUALS", "several compatible actual declarations"));
    assert!(is_spurious(dir.path(), &file, "EXPECT_AND_ACTUAL_IN_THE_SAME_MODULE", "same module"));
}

#[test]
fn other_codes_are_never_spurious() {
    let dir = module();
    let file = dir.path().join("src/commonMain/kotlin/Platform.kt");
    assert!(!is_spurious(dir.path(), &file, "TYPE_MISMATCH", ACROSS_SETS));
    assert!(!is_spurious(dir.path(), &file, "UNRESOLVED_REFERENCE", ""));
}

#[test]
fn files_outside_a_multiplatform_module_keep_every_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle.kts"), "plugins { kotlin(\"jvm\") }").unwrap();
    let file = dir.path().join("src/main/kotlin/A.kt");
    assert!(!is_spurious(dir.path(), &file, "CONFLICTING_OVERLOADS", ACROSS_SETS));
    assert!(!is_spurious(dir.path(), &file, "AMBIGUOUS_ACTUALS", ""));
}
