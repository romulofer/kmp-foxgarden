use super::*;

#[test]
fn the_extension_contributes_no_language_and_needs_no_capability() {
    let extension = KmpExtension::new();
    let contributions = extension.contributions();
    assert!(contributions.languages.is_empty() && contributions.grammars.is_empty());
    assert!(extension.capabilities().is_empty());
}

#[test]
fn it_answers_source_set_questions_through_the_extension_api() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle.kts"), "plugins { kotlin(\"multiplatform\") }").unwrap();
    let file = dir.path().join("src/commonMain/kotlin/A.kt");
    assert_eq!(KmpExtension::new().source_set(dir.path(), &file).as_deref(), Some("commonMain"));
}

#[test]
fn the_new_project_template_is_offered_unless_the_host_leaves_it_out() {
    assert_eq!(KmpExtension::new().contributions().scaffolds.len(), 1);
    assert!(KmpExtension::without_scaffold().contributions().scaffolds.is_empty());
}
