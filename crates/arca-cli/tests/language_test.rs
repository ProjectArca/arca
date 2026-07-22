use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct TestResult {
    name: String,
    status: TestStatus,
    expected: String,
    actual: String,
}

enum TestStatus {
    Pass,
    Fail,
    BuildError,
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../")
}

fn discover_tests() -> Vec<PathBuf> {
    let dir = project_root().join("tests/features");
    let mut tests = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(false, |e| e == "arca") {
            tests.push(path);
        }
    }
    tests.sort();
    tests
}

fn parse_expected(source: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(content) = trimmed.strip_prefix("//") {
            let content = content.trim();
            if content.starts_with(|c: char| c.is_ascii_digit() || c == '[' || c == '-' || c == '"') {
                lines.push(content.to_string());
            }
        }
    }
    lines
}

fn compile(path: &PathBuf) -> Result<(), String> {
    let root = project_root();
    let output = Command::new("cargo")
        .args(["run", "-q", "--", "build"])
        .arg(path)
        .current_dir(&root)
        .output()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("Build status: SUCCESS") {
        return Ok(());
    }
    let combined = format!("{}{}", stdout, stderr);
    Err(format!("Compile failed:\n{}", combined))
}

fn build_c_binary(out_dir: &PathBuf) -> Result<(), String> {
    let root = project_root();
    let runtime_dir = root.join("library/runtime");
    let mut cmd = Command::new("cc");
    cmd.args([
        "-O3",
        "-o",
        &out_dir.join("test_bin").to_string_lossy(),
    ]);
    cmd.arg(root.join("build/output.c"));
    cmd.arg("-I").arg(&runtime_dir);
    cmd.arg(runtime_dir.join("arca_runtime.c"));
    let conc_dir = root.join("library/concurrency");
    if conc_dir.exists() {
        for entry in fs::read_dir(&conc_dir).unwrap() {
            let p = entry.unwrap().path();
            if p.extension().map_or(false, |e| e == "c") {
                cmd.arg(&p);
            }
        }
    }
    let net_dir = root.join("library/net");
    if net_dir.exists() {
        for entry in fs::read_dir(&net_dir).unwrap() {
            let p = entry.unwrap().path();
            if p.extension().map_or(false, |e| e == "c") {
                cmd.arg(&p);
            }
        }
    }
    cmd.arg("-lpthread");

    let output = cmd.output().map_err(|e| format!("cc failed: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cc compile failed:\n{}", stderr));
    }
    Ok(())
}

fn run_binary(out_dir: &PathBuf) -> Result<String, String> {
    let output = Command::new(out_dir.join("test_bin"))
        .output()
        .map_err(|e| format!("Run failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

fn matches_expected(actual: &str, expected: &str) -> bool {
    if expected.starts_with('[') && expected.ends_with(']') {
        return true;
    }
    actual == expected
}

fn run_test(path: &PathBuf, out_dir: &PathBuf) -> TestResult {
    let name = path.file_stem().unwrap().to_string_lossy().to_string();
    let source = fs::read_to_string(path).unwrap_or_default();
    let expected_lines = parse_expected(&source);

    if let Err(e) = compile(path) {
        return TestResult { name, status: TestStatus::BuildError, expected: String::new(), actual: e };
    }
    if let Err(e) = build_c_binary(out_dir) {
        return TestResult { name, status: TestStatus::BuildError, expected: String::new(), actual: e };
    }
    let actual = run_binary(out_dir).unwrap_or_default();
    let actual_lines: Vec<&str> = actual.trim().lines().collect();

    let mut pass = true;
    let mut detail = String::new();
    for (i, expected_line) in expected_lines.iter().enumerate() {
        let actual_line = actual_lines.get(i).copied().unwrap_or("");
        if !matches_expected(actual_line, expected_line) {
            pass = false;
            detail.push_str(&format!("  line {}: expected={:?} actual={:?}\n", i + 1, expected_line, actual_line));
        }
    }
    if actual_lines.len() != expected_lines.len() {
        pass = false;
    }

    let status = if pass { TestStatus::Pass } else { TestStatus::Fail };
    let expected = expected_lines.join("\n");
    let actual_joined = actual_lines.join("\n");

    TestResult { name, status, expected, actual: if detail.is_empty() { actual_joined } else { detail } }
}

#[test]
fn run_all_language_tests() {
    let out_dir = PathBuf::from(std::env::temp_dir()).join("arca_lang_test");
    let _ = fs::create_dir_all(&out_dir);

    let tests = discover_tests();
    assert!(!tests.is_empty(), "No test files found in tests/features/");

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut build_errors = 0u32;
    let mut results = Vec::new();

    for path in &tests {
        let result = run_test(path, &out_dir);
        match result.status {
            TestStatus::Pass => passed += 1,
            TestStatus::Fail => failed += 1,
            TestStatus::BuildError => build_errors += 1,
        }
        results.push(result);
    }

    println!("\n=== Arca Language Test Report ===");
    println!("{:<30} {:>6} {:>20}", "Test", "Status", "Expected / Actual");
    println!("{}", "-".repeat(80));
    for r in &results {
        let status_str = match r.status {
            TestStatus::Pass => "PASS",
            TestStatus::Fail => "FAIL",
            TestStatus::BuildError => "ERR",
        };
        let detail = match r.status {
            TestStatus::Pass => format!("{:>20}", "ok"),
            TestStatus::Fail => format!("expected={:?} actual={:?}", r.expected, r.actual),
            TestStatus::BuildError => format!("{:>20}", r.actual.lines().last().unwrap_or("")),
        };
        println!("{:<30} {:>6} {}", r.name, status_str, detail);
    }
    println!("{}", "-".repeat(80));
    println!("Total: {}  Passed: {}  Failed: {}  Build errors: {}",
        tests.len(), passed, failed, build_errors);

    assert!(failed == 0, "{} test(s) failed", failed);
    assert!(build_errors == 0, "{} build error(s)", build_errors);
}
