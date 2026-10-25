//! The test reports a multiplatform Gradle task leaves behind (`PLAN.md`
//! Track 30 Phase 3).
//!
//! Gradle writes one JUnit-style `TEST-<class>.xml` per test class into
//! `build/test-results/<task>/`. A failing case is turned into a location by
//! looking for the class's source under the module's source sets, since
//! multiplatform tests sit in `src/jvmTest/kotlin`, `src/commonTest/kotlin`
//! and so on rather than in a single `src/test`.

use std::path::{Path, PathBuf};

use fg_extension::{TaskTestResult, TestCase, TestFailureLocation, TestOutcome};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// Every test reported by the `TEST-*.xml` files in `dirs`. A file that cannot
/// be read or parsed is skipped, so one stale report does not hide the rest.
pub fn task_test_results(dirs: &[PathBuf]) -> Vec<TaskTestResult> {
    let mut results = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("TEST-") && n.ends_with(".xml"))
            })
            .collect();
        files.sort();
        for file in files {
            let Ok(xml) = std::fs::read_to_string(&file) else { continue };
            let Ok(cases) = parse(&xml) else { continue };
            let module = module_of(dir);
            for case in cases {
                let at = matches!(case.outcome, TestOutcome::Failed | TestOutcome::Errored)
                    .then(|| module.as_deref().and_then(|module| failure_location(module, &case)))
                    .flatten();
                results.push(TaskTestResult { case, at });
            }
        }
    }
    results
}

/// The module a report directory (`<module>/build/test-results/<task>`)
/// belongs to.
fn module_of(report_dir: &Path) -> Option<PathBuf> {
    Some(report_dir.parent()?.parent()?.parent()?.to_path_buf())
}

fn attr(start: &BytesStart<'_>, name: &[u8]) -> Result<Option<String>, String> {
    for a in start.attributes() {
        let a = a.map_err(|e| e.to_string())?;
        if a.key.as_ref() == name {
            return a.unescape_value().map(|v| Some(v.into_owned())).map_err(|e| e.to_string());
        }
    }
    Ok(None)
}

fn case_from(start: &BytesStart<'_>) -> Result<TestCase, String> {
    let name = attr(start, b"name")?.unwrap_or_default();
    Ok(TestCase {
        classname: attr(start, b"classname")?.unwrap_or_default(),
        name: name.strip_suffix("()").unwrap_or(&name).to_string(),
        outcome: TestOutcome::Passed,
        detail: None,
    })
}

fn parse(xml: &str) -> Result<Vec<TestCase>, String> {
    let mut reader = Reader::from_str(xml);
    let mut cases = Vec::new();
    let mut pending: Option<TestCase> = None;
    let mut capturing = false;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Eof => break,
            Event::Start(e) if e.local_name().as_ref() == b"testcase" => pending = Some(case_from(&e)?),
            Event::Empty(e) if e.local_name().as_ref() == b"testcase" => cases.push(case_from(&e)?),
            Event::Start(e) | Event::Empty(e) if matches!(e.local_name().as_ref(), b"failure" | b"error") => {
                if let Some(case) = pending.as_mut() {
                    case.outcome = if e.local_name().as_ref() == b"failure" { TestOutcome::Failed } else { TestOutcome::Errored };
                    case.detail = Some(attr(&e, b"message")?.unwrap_or_default());
                    capturing = true;
                }
            }
            Event::Empty(e) if e.local_name().as_ref() == b"skipped" => {
                if let Some(case) = pending.as_mut() {
                    case.outcome = TestOutcome::Skipped;
                }
            }
            Event::Start(e) if e.local_name().as_ref() == b"skipped" => {
                if let Some(case) = pending.as_mut() {
                    case.outcome = TestOutcome::Skipped;
                }
            }
            Event::Text(t) if capturing => {
                let text = t.decode().map_err(|e| e.to_string())?;
                if let Some(detail) = pending.as_mut().and_then(|c| c.detail.as_mut()) {
                    detail.push_str(&text);
                }
            }
            Event::CData(t) if capturing => {
                let text = t.decode().map_err(|e| e.to_string())?;
                if let Some(detail) = pending.as_mut().and_then(|c| c.detail.as_mut()) {
                    detail.push_str(&text);
                }
            }
            Event::End(e) if matches!(e.local_name().as_ref(), b"failure" | b"error") => capturing = false,
            Event::End(e) if e.local_name().as_ref() == b"testcase" => {
                if let Some(case) = pending.take() {
                    cases.push(case);
                }
            }
            _ => {}
        }
    }
    Ok(cases)
}

/// Where a failing `case` lives: its class's source file in any source set of
/// `module`, and the line its stack trace blames in that file (line 1 when the
/// trace names none).
fn failure_location(module: &Path, case: &TestCase) -> Option<TestFailureLocation> {
    let relative = case.classname.replace('.', "/");
    let class_file = relative.rsplit('/').next().unwrap_or(&relative).to_string();
    let top_level = format!("{relative}.kt");
    for set in std::fs::read_dir(module.join("src")).ok()?.flatten().map(|e| e.path()) {
        for dir in ["kotlin", "java"] {
            let path = set.join(dir).join(&top_level);
            if path.is_file() {
                let line = case.detail.as_deref().and_then(|d| blamed_line(d, &class_file)).unwrap_or(1);
                return Some(TestFailureLocation { path, line });
            }
        }
    }
    None
}

/// The line a stack frame names in `<class>.kt`, taking the first such frame
/// since those above it are the assertion library's own.
fn blamed_line(detail: &str, class_name: &str) -> Option<usize> {
    let marker = format!("({class_name}.kt:");
    let start = detail.find(&marker)? + marker.len();
    let rest = &detail[start..];
    rest[..rest.find(')')?].trim().parse().ok()
}

#[cfg(test)]
#[path = "reports_test.rs"]
mod reports_test;
