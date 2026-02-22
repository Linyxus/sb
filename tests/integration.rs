use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::sync::Mutex;

/// Integration tests must run sequentially because they share a global Maven/JAR cache.
/// Parallel resolution of the same artifacts can cause corruption.
static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn sb_binary() -> PathBuf {
    // cargo sets this env var when running tests
    PathBuf::from(env!("CARGO_BIN_EXE_sb"))
}

fn discover_projects(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return vec![];
    }
    let mut projects: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("sb.toml").exists())
        .collect();
    projects.sort();
    projects
}

fn run_sb(project_dir: &Path, args: &[&str]) -> std::process::Output {
    // Use a temp dir for .sb to avoid polluting the source tree
    let tmp = tempfile::tempdir().unwrap();
    let work_dir = tmp.path().join(project_dir.file_name().unwrap());

    // Copy project to temp dir
    copy_dir_all(project_dir, &work_dir).unwrap();

    Command::new(sb_binary())
        .args(args)
        .current_dir(&work_dir)
        .output()
        .expect("failed to run sb")
}

fn run_sb_in(work_dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(sb_binary())
        .args(args)
        .current_dir(work_dir)
        .output()
        .expect("failed to run sb")
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

#[test]
fn pos_projects_build() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let pos_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pos");
    let projects = discover_projects(&pos_dir);
    assert!(!projects.is_empty(), "no positive test projects found in tests/pos/");

    for project in &projects {
        let name = project.file_name().unwrap().to_string_lossy();
        eprintln!("testing pos/{name} ...");

        let output = run_sb(project, &["build"]);
        assert!(
            output.status.success(),
            "pos/{name}: sb build failed\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

#[test]
fn pos_projects_run() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let pos_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pos");
    let projects = discover_projects(&pos_dir);

    for project in &projects {
        let name = project.file_name().unwrap().to_string_lossy();

        // Only test run if main-class is set
        let toml_content = std::fs::read_to_string(project.join("sb.toml")).unwrap();
        if !toml_content.contains("main-class") {
            continue;
        }

        eprintln!("testing pos/{name} run ...");

        let output = run_sb(project, &["run"]);
        assert!(
            output.status.success(),
            "pos/{name}: sb run failed\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

#[test]
fn run_with_cli_args() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pos/cli_args");
    let output = run_sb(&project, &["run", "--", "hello", "world", "foo"]);
    assert!(
        output.status.success(),
        "sb run with args failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello\nworld\nfoo"),
        "expected CLI args in output, got: {stdout}",
    );
}

#[test]
fn asm_produces_runnable_jar() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pos/cli_args");
    let tmp = tempfile::tempdir().unwrap();
    let work_dir = tmp.path().join("cli_args");
    copy_dir_all(&project, &work_dir).unwrap();

    let output = run_sb_in(&work_dir, &["asm"]);
    assert!(
        output.status.success(),
        "sb asm failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // Run the assembled JAR
    let jar_path = work_dir.join(".sb/cli_args-0.1.0-assembly.jar");
    assert!(jar_path.exists(), "assembly jar not found at {}", jar_path.display());

    let run_output = Command::new("java")
        .args(["-jar", jar_path.to_str().unwrap(), "hello", "world", "foo"])
        .output()
        .expect("failed to run java");
    assert!(
        run_output.status.success(),
        "java -jar failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr),
    );
    let stdout = String::from_utf8_lossy(&run_output.stdout);
    assert!(
        stdout.contains("hello\nworld\nfoo"),
        "expected CLI args in output, got: {stdout}",
    );
}

#[test]
fn neg_projects_fail() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let neg_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/neg");
    let projects = discover_projects(&neg_dir);

    if projects.is_empty() {
        eprintln!("no negative test projects in tests/neg/, skipping");
        return;
    }

    for project in &projects {
        let name = project.file_name().unwrap().to_string_lossy();
        eprintln!("testing neg/{name} ...");

        let output = run_sb(project, &["build"]);
        assert!(
            !output.status.success(),
            "neg/{name}: sb build should have failed but succeeded",
        );
    }
}

#[test]
fn tasty_roundtrip() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pos/firstproj");
    let tmp = tempfile::tempdir().unwrap();
    let work_dir = tmp.path().join("firstproj");
    copy_dir_all(&project, &work_dir).unwrap();

    // Build first
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(
        output.status.success(),
        "sb build failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    // Find .tasty files
    let classes_dir = work_dir.join(".sb/classes");
    let tasty_files: Vec<PathBuf> = walkdir::WalkDir::new(&classes_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "tasty"))
        .map(|e| e.path().to_path_buf())
        .collect();
    assert!(!tasty_files.is_empty(), "no .tasty files found after build");

    // Run sb tasty on each .tasty file
    for tasty_file in &tasty_files {
        let output = run_sb_in(&work_dir, &["tasty", tasty_file.to_str().unwrap()]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "sb tasty failed for {}\nstdout: {}\nstderr: {}",
            tasty_file.display(),
            stdout,
            stderr,
        );
        assert!(
            stdout.contains("TASTy"),
            "expected 'TASTy' in output for {}, got: {}",
            tasty_file.display(),
            stdout,
        );
    }
}

// ====================================================================
// Helper: set up incremental test project in a temp dir and do initial build
// ====================================================================

fn setup_incremental() -> (tempfile::TempDir, PathBuf) {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pos/incremental");
    let tmp = tempfile::tempdir().unwrap();
    let work_dir = tmp.path().join("incremental");
    copy_dir_all(&project, &work_dir).unwrap();

    // Initial full build
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(
        output.status.success(),
        "initial build failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Compiling 3 source files"),
        "expected full compile of 3 files, got: {stderr}",
    );

    (tmp, work_dir)
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

// ====================================================================
// 1. Noop rebuild: nothing changed → "Nothing to compile"
// ====================================================================
#[test]
fn incremental_noop_rebuild() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    assert!(
        stderr_of(&output).contains("Nothing to compile"),
        "expected noop, got: {}",
        stderr_of(&output),
    );
}

// ====================================================================
// 2. Two consecutive noops: stable after first noop
// ====================================================================
#[test]
fn incremental_double_noop() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    let o1 = run_sb_in(&work_dir, &["build"]);
    assert!(stderr_of(&o1).contains("Nothing to compile"));
    let o2 = run_sb_in(&work_dir, &["build"]);
    assert!(stderr_of(&o2).contains("Nothing to compile"));
}

// ====================================================================
// 3. Body-only change: only that file recompiled, no cascade
// ====================================================================
#[test]
fn incremental_body_only_change() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    std::fs::write(
        work_dir.join("src/main/scala/Middle.scala"),
        "object Middle:\n  def process(b: Base): String = s\"Value: ${b.x}\"\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("Compiling 1 source file..."),
        "expected 1 file, got: {}",
        stderr_of(&output),
    );

    // Verify output changed
    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("Value: 42"), "got: {}", stdout_of(&output));
}

// ====================================================================
// 4. API change cascades to dependents
// ====================================================================
#[test]
fn incremental_api_change_cascades() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Add a field to Base (API change)
    std::fs::write(
        work_dir.join("src/main/scala/Base.scala"),
        "case class Base(x: Int, y: Int = 0)\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    assert!(
        !stderr_of(&output).contains("Nothing to compile"),
        "expected recompilation, got: {}",
        stderr_of(&output),
    );

    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("Base: 42"));
}

// ====================================================================
// 5. Add standalone file: only new file compiled
// ====================================================================
#[test]
fn incremental_add_standalone_file() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    std::fs::write(
        work_dir.join("src/main/scala/Extra.scala"),
        "object Extra:\n  def greet(): String = \"hello\"\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("Compiling 1 source file..."),
        "expected 1 file, got: {}",
        stderr_of(&output),
    );
}

// ====================================================================
// 6. Delete an unused file: no recompilation needed
// ====================================================================
#[test]
fn incremental_delete_unused_file() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Add a standalone file first
    std::fs::write(
        work_dir.join("src/main/scala/Unused.scala"),
        "object Unused:\n  val x = 42\n",
    ).unwrap();
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());

    // Now delete it
    std::fs::remove_file(work_dir.join("src/main/scala/Unused.scala")).unwrap();

    // Build should succeed without recompiling anything (no one depends on Unused)
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    // Nothing depends on it so nothing to recompile — but deletion is detected,
    // so we get an empty recompile set (no "Compiling" message)
    assert!(
        !stderr_of(&output).contains("Compiling 3 source file"),
        "should not full-recompile, got: {}",
        stderr_of(&output),
    );

    // Project still runs correctly
    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
}

// ====================================================================
// 7. Delete depended-on file: compilation fails
// ====================================================================
#[test]
fn incremental_delete_depended_file_fails() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Delete Base.scala which Middle depends on
    std::fs::remove_file(work_dir.join("src/main/scala/Base.scala")).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(
        !output.status.success(),
        "build should fail after deleting depended-on file",
    );
}

// ====================================================================
// 8. Change multiple files' bodies at once: no cascade
// ====================================================================
#[test]
fn incremental_multiple_body_changes() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Change bodies of Middle and Top (no API change)
    std::fs::write(
        work_dir.join("src/main/scala/Middle.scala"),
        "object Middle:\n  def process(b: Base): String = s\"Got: ${b.x}\"\n",
    ).unwrap();
    std::fs::write(
        work_dir.join("src/main/scala/Top.scala"),
        "@main def top(): Unit =\n  println(\">>\" + Middle.process(Base(42)))\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("Compiling 2 source files"),
        "expected 2 files, got: {}",
        stderr_of(&output),
    );

    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains(">>Got: 42"));
}

// ====================================================================
// 9. Whitespace-only change: file hash changes, recompile that file only
// ====================================================================
#[test]
fn incremental_whitespace_change() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Add trailing newlines to Middle.scala (whitespace only)
    std::fs::write(
        work_dir.join("src/main/scala/Middle.scala"),
        "object Middle:\n  def process(b: Base): String = s\"Base: ${b.x}\"\n\n\n\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    assert!(
        stderr_of(&output).contains("Compiling 1 source file..."),
        "expected 1 file for whitespace change, got: {}",
        stderr_of(&output),
    );

    // Noop after
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(stderr_of(&output).contains("Nothing to compile"));
}

// ====================================================================
// 10. Add new method to existing object (API change)
// ====================================================================
#[test]
fn incremental_add_method_api_change() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Add a new public method to Middle
    std::fs::write(
        work_dir.join("src/main/scala/Middle.scala"),
        "object Middle:\n  def process(b: Base): String = s\"Base: ${b.x}\"\n  def extra(): Int = 99\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    // Middle changed; API change may or may not cascade depending on hash
    assert!(
        !stderr_of(&output).contains("Nothing to compile"),
        "expected recompilation",
    );

    // Still runs
    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
}

// ====================================================================
// 11. Change return type (API change) — return Int instead of String
// ====================================================================
#[test]
fn incremental_change_return_type() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Change Middle.process return type from String to Int and update Top accordingly
    std::fs::write(
        work_dir.join("src/main/scala/Middle.scala"),
        "object Middle:\n  def process(b: Base): Int = b.x * 2\n",
    ).unwrap();
    std::fs::write(
        work_dir.join("src/main/scala/Top.scala"),
        "@main def top(): Unit =\n  val n: Int = Middle.process(Base(42))\n  println(n)\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("Compiling 2 source files"),
        "expected 2 files, got: {}",
        stderr_of(&output),
    );

    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success(), "run failed: {}", stderr_of(&output));
    assert!(stdout_of(&output).contains("84"), "expected 84, got: {}", stdout_of(&output));
}

// ====================================================================
// 12. Add new file that depends on existing types
// ====================================================================
#[test]
fn incremental_add_file_using_existing() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    std::fs::write(
        work_dir.join("src/main/scala/Helper.scala"),
        "object Helper:\n  def double(b: Base): Base = Base(b.x * 2)\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("Compiling 1 source file..."),
        "expected only 1 file compiled, got: {}",
        stderr_of(&output),
    );
}

// ====================================================================
// 13. Three sequential incremental builds with small changes
// ====================================================================
#[test]
fn incremental_sequential_builds() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Build 1: body change to Middle
    std::fs::write(
        work_dir.join("src/main/scala/Middle.scala"),
        "object Middle:\n  def process(b: Base): String = s\"A: ${b.x}\"\n",
    ).unwrap();
    let o1 = run_sb_in(&work_dir, &["build"]);
    assert!(o1.status.success());
    assert!(stderr_of(&o1).contains("Compiling 1 source file"));

    // Build 2: body change to Top
    std::fs::write(
        work_dir.join("src/main/scala/Top.scala"),
        "@main def top(): Unit = println(\"=\" + Middle.process(Base(42)))\n",
    ).unwrap();
    let o2 = run_sb_in(&work_dir, &["build"]);
    assert!(o2.status.success());
    assert!(stderr_of(&o2).contains("Compiling 1 source file"));

    // Build 3: noop
    let o3 = run_sb_in(&work_dir, &["build"]);
    assert!(stderr_of(&o3).contains("Nothing to compile"));

    // Verify final output
    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("=A: 42"));
}

// ====================================================================
// 14. Clean then rebuild: full recompile after clean
// ====================================================================
#[test]
fn incremental_clean_then_rebuild() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    let output = run_sb_in(&work_dir, &["clean"]);
    assert!(output.status.success());

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    assert!(
        stderr_of(&output).contains("Compiling 3 source files"),
        "expected full recompile after clean, got: {}",
        stderr_of(&output),
    );

    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
}

// ====================================================================
// 15. Change only the main-class file (Top.scala) body
// ====================================================================
#[test]
fn incremental_change_main_class_body() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    std::fs::write(
        work_dir.join("src/main/scala/Top.scala"),
        "@main def top(): Unit =\n  val result = Middle.process(Base(99))\n  println(s\"Result: $result\")\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    assert!(
        stderr_of(&output).contains("Compiling 1 source file"),
        "expected 1 file, got: {}",
        stderr_of(&output),
    );

    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("Result: Base: 99"));
}

// ====================================================================
// 16. Simultaneous add + modify
// ====================================================================
#[test]
fn incremental_add_and_modify() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Add a new file and change Middle body simultaneously
    std::fs::write(
        work_dir.join("src/main/scala/NewFile.scala"),
        "object NewFile:\n  val msg = \"new\"\n",
    ).unwrap();
    std::fs::write(
        work_dir.join("src/main/scala/Middle.scala"),
        "object Middle:\n  def process(b: Base): String = s\"Modified: ${b.x}\"\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("Compiling 2 source files"),
        "expected 2 files, got: {}",
        stderr_of(&output),
    );
}

// ====================================================================
// 17. Modify all files' bodies at once
// ====================================================================
#[test]
fn incremental_modify_all_bodies() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    std::fs::write(
        work_dir.join("src/main/scala/Base.scala"),
        "case class Base(x: Int) // comment\n",
    ).unwrap();
    std::fs::write(
        work_dir.join("src/main/scala/Middle.scala"),
        "object Middle:\n  def process(b: Base): String = s\"X=${b.x}\"\n",
    ).unwrap();
    std::fs::write(
        work_dir.join("src/main/scala/Top.scala"),
        "@main def top(): Unit = println(Middle.process(Base(42))) // changed\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    // All 3 changed, round 1 compiles all 3
    assert!(
        stderr_of(&output).contains("Compiling 3 source files"),
        "expected 3 files, got: {}",
        stderr_of(&output),
    );

    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("X=42"));
}

// ====================================================================
// 18. Add file then delete it: back to original state
// ====================================================================
#[test]
fn incremental_add_then_delete() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Add
    std::fs::write(
        work_dir.join("src/main/scala/Temp.scala"),
        "object Temp:\n  val v = 1\n",
    ).unwrap();
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    assert!(stderr_of(&output).contains("Compiling 1 source file"));

    // Delete
    std::fs::remove_file(work_dir.join("src/main/scala/Temp.scala")).unwrap();
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    // Should not require full recompile
    assert!(
        !stderr_of(&output).contains("Compiling 3 source file"),
        "should not full recompile, got: {}",
        stderr_of(&output),
    );

    // Original program still works
    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("Base: 42"));
}

// ====================================================================
// 19. Two consecutive API changes
// ====================================================================
#[test]
fn incremental_two_api_changes() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // First API change: add field to Base
    std::fs::write(
        work_dir.join("src/main/scala/Base.scala"),
        "case class Base(x: Int, y: Int = 0)\n",
    ).unwrap();
    let o1 = run_sb_in(&work_dir, &["build"]);
    assert!(o1.status.success(), "first API change build failed: {}", stderr_of(&o1));
    let run1 = run_sb_in(&work_dir, &["run"]);
    assert!(run1.status.success());

    // Second API change: add another field
    std::fs::write(
        work_dir.join("src/main/scala/Base.scala"),
        "case class Base(x: Int, y: Int = 0, z: String = \"\")\n",
    ).unwrap();
    let o2 = run_sb_in(&work_dir, &["build"]);
    assert!(o2.status.success(), "second API change build failed: {}", stderr_of(&o2));

    let run2 = run_sb_in(&work_dir, &["run"]);
    assert!(run2.status.success());
    assert!(stdout_of(&run2).contains("Base: 42"));
}

// ====================================================================
// 20. Run after incremental build works correctly
// ====================================================================
#[test]
fn incremental_run_after_rebuild() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Verify initial run
    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("Base: 42"));

    // Body change
    std::fs::write(
        work_dir.join("src/main/scala/Middle.scala"),
        "object Middle:\n  def process(b: Base): String = s\"NEW: ${b.x}\"\n",
    ).unwrap();
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());

    // Run picks up the change
    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("NEW: 42"), "got: {}", stdout_of(&output));
}

// ====================================================================
// 21. Rewrite file with identical content: noop
// ====================================================================
#[test]
fn incremental_rewrite_same_content() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Read and rewrite Middle.scala with exact same content
    let content = std::fs::read_to_string(work_dir.join("src/main/scala/Middle.scala")).unwrap();
    std::fs::write(work_dir.join("src/main/scala/Middle.scala"), &content).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    assert!(
        stderr_of(&output).contains("Nothing to compile"),
        "expected noop for identical content, got: {}",
        stderr_of(&output),
    );
}

// ====================================================================
// 22. Add file in subdirectory
// ====================================================================
#[test]
fn incremental_add_file_in_subdir() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    let subdir = work_dir.join("src/main/scala/sub");
    std::fs::create_dir_all(&subdir).unwrap();
    std::fs::write(
        subdir.join("Deep.scala"),
        "package sub\n\nobject Deep:\n  val value = 123\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success(), "build failed: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("Compiling 1 source file"),
        "expected 1 file, got: {}",
        stderr_of(&output),
    );

    // Original still works
    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
}

// ====================================================================
// 23. Leaf file change (Top.scala): no cascade since nothing depends on it
// ====================================================================
#[test]
fn incremental_leaf_change_no_cascade() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Change Top.scala body — it's a leaf, nothing depends on it
    std::fs::write(
        work_dir.join("src/main/scala/Top.scala"),
        "@main def top(): Unit =\n  println(\"Hello \" + Middle.process(Base(7)))\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    assert!(
        stderr_of(&output).contains("Compiling 1 source file..."),
        "expected exactly 1 file (leaf), got: {}",
        stderr_of(&output),
    );

    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("Hello Base: 7"));
}

// ====================================================================
// 24. Incremental state survives across build+run cycles
// ====================================================================
#[test]
fn incremental_state_persists_after_run() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    // Run (which also compiles)
    let output = run_sb_in(&work_dir, &["run"]);
    assert!(output.status.success());

    // Build should be noop
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    assert!(
        stderr_of(&output).contains("Nothing to compile"),
        "expected noop after run, got: {}",
        stderr_of(&output),
    );
}

// ====================================================================
// 25. Change comment only (inside body): 1 file recompiled
// ====================================================================
#[test]
fn incremental_comment_change() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_tmp, work_dir) = setup_incremental();

    std::fs::write(
        work_dir.join("src/main/scala/Base.scala"),
        "// This is a comment\ncase class Base(x: Int)\n",
    ).unwrap();

    let output = run_sb_in(&work_dir, &["build"]);
    assert!(output.status.success());
    // Hash changed → 1 file recompiled
    assert!(
        stderr_of(&output).contains("Compiling 1 source file"),
        "expected 1 file for comment change, got: {}",
        stderr_of(&output),
    );

    // Still noop after
    let output = run_sb_in(&work_dir, &["build"]);
    assert!(stderr_of(&output).contains("Nothing to compile"));
}

#[test]
fn scala2_rejected() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/neg/scala2");
    let output = run_sb(&project, &["build"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("only Scala 3.x is supported"),
        "expected Scala 3 error message, got: {stderr}",
    );
}
