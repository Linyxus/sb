use std::path::{Path, PathBuf};
use std::process::Command;

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
fn neg_projects_fail() {
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
