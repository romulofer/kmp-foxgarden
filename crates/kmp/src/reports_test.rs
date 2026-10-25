use super::*;

const REPORT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuite name="app.GreeterTest" tests="3" skipped="1" failures="1" errors="0">
  <testcase name="greets()" classname="app.GreeterTest" time="0.01"/>
  <testcase name="shouts" classname="app.GreeterTest" time="0.00">
    <failure message="expected:&lt;HI&gt; but was:&lt;hi&gt;" type="AssertionFailedError">AssertionFailedError: expected
	at org.junit.Assert.fail(Assert.java:1)
	at app.GreeterTest.shouts(GreeterTest.kt:14)
</failure>
  </testcase>
  <testcase name="later" classname="app.GreeterTest"><skipped/></testcase>
</testsuite>
"#;

fn module() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let reports = dir.path().join("build/test-results/jvmTest");
    std::fs::create_dir_all(&reports).unwrap();
    std::fs::write(reports.join("TEST-app.GreeterTest.xml"), REPORT).unwrap();
    std::fs::write(reports.join("binary.bin"), "not a report").unwrap();
    let source = dir.path().join("src/jvmTest/kotlin/app/GreeterTest.kt");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(source, "class GreeterTest\n").unwrap();
    (dir, reports)
}

#[test]
fn reads_every_case_with_its_outcome() {
    let (_dir, reports) = module();
    let results = task_test_results(&[reports]);
    let outcomes: Vec<(&str, &TestOutcome)> = results.iter().map(|r| (r.case.name.as_str(), &r.case.outcome)).collect();
    assert_eq!(
        outcomes,
        [("greets", &TestOutcome::Passed), ("shouts", &TestOutcome::Failed), ("later", &TestOutcome::Skipped)]
    );
    assert!(results[1].case.detail.as_deref().unwrap().starts_with("expected:<HI> but was:<hi>"));
}

#[test]
fn a_failure_points_at_the_line_its_trace_blames_in_the_tests_source_set() {
    let (dir, reports) = module();
    let results = task_test_results(&[reports]);
    let at = results[1].at.as_ref().expect("the failing case is located");
    assert_eq!(at.path, dir.path().join("src/jvmTest/kotlin/app/GreeterTest.kt"));
    assert_eq!(at.line, 14);
    assert!(results[0].at.is_none(), "passing cases need no location");
}

#[test]
fn a_missing_directory_or_a_broken_report_gives_nothing_rather_than_failing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(task_test_results(&[dir.path().join("build/test-results/none")]).is_empty());
    let reports = dir.path().join("build/test-results/jsTest");
    std::fs::create_dir_all(&reports).unwrap();
    std::fs::write(reports.join("TEST-x.xml"), "<testsuite><testcase").unwrap();
    assert!(task_test_results(&[reports]).is_empty());
}

#[test]
fn a_failure_whose_source_cannot_be_found_is_still_reported() {
    let (dir, reports) = module();
    std::fs::remove_file(dir.path().join("src/jvmTest/kotlin/app/GreeterTest.kt")).unwrap();
    let results = task_test_results(&[reports]);
    assert_eq!(results[1].case.outcome, TestOutcome::Failed);
    assert!(results[1].at.is_none());
}
