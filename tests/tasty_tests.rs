use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn sb_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sb"))
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

/// Build the tasty test project once and return the work_dir path.
/// The temp dir is leaked intentionally so it persists for all tests.
fn get_work_dir() -> &'static PathBuf {
    static WORK_DIR: OnceLock<PathBuf> = OnceLock::new();
    WORK_DIR.get_or_init(|| {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/tasty");
        let tmp = tempfile::tempdir().unwrap();
        let work_dir = tmp.path().join("tasty");
        copy_dir_all(&project, &work_dir).unwrap();

        let output = Command::new(sb_binary())
            .args(["build"])
            .current_dir(&work_dir)
            .output()
            .expect("failed to run sb build");
        assert!(
            output.status.success(),
            "sb build failed for tasty test project\nstderr: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        // Leak the TempDir so the directory persists for all tests
        let _ = std::mem::ManuallyDrop::new(tmp);
        work_dir
    })
}

/// Run `sb tasty` on a .tasty file and return stdout.
fn run_tasty(work_dir: &Path, tasty_file: &str) -> String {
    let file_path = work_dir.join(format!(".sb/classes/{}", tasty_file));
    assert!(
        file_path.exists(),
        "tasty file not found: {}",
        file_path.display()
    );
    let output = Command::new(sb_binary())
        .args(["tasty", file_path.to_str().unwrap()])
        .current_dir(work_dir)
        .output()
        .expect("failed to run sb tasty");
    assert!(
        output.status.success(),
        "sb tasty failed for {}\nstderr: {}",
        tasty_file,
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout).unwrap()
}

/// Find all .tasty files in .sb/classes/
fn find_tasty_files(work_dir: &Path) -> Vec<String> {
    let classes_dir = work_dir.join(".sb/classes");
    walkdir::WalkDir::new(&classes_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "tasty"))
        .map(|e| {
            e.path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect()
}

// ============================================================
// Tests
// ============================================================

/// All .tasty files produced by the project must parse without error.
#[test]
fn tasty_parse_all_files() {
    let work_dir = get_work_dir();
    let files = find_tasty_files(&work_dir);
    assert!(
        files.len() >= 20,
        "expected at least 20 .tasty files, found {}",
        files.len()
    );

    for file in &files {
        let output = run_tasty(&work_dir, file);
        assert!(
            output.contains("TASTy file"),
            "{}: missing 'TASTy file' header",
            file
        );
        assert!(
            output.contains("Names ("),
            "{}: missing name table output",
            file
        );
    }
}

/// All files report the correct TASTy version and tooling.
#[test]
fn tasty_version_and_tooling() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Empty.tasty");
    assert!(output.contains("version: 28."));
    assert!(output.contains("tooling: Scala 3.6.4"));
}

/// Empty object: minimal TASTy output.
#[test]
fn tasty_empty_object() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Empty.tasty");
    assert!(output.contains("Empty"), "should contain name 'Empty'");
    assert!(output.contains("PACKAGE"), "should contain PACKAGE node");
    assert!(output.contains("TYPEDEF"), "should contain TYPEDEF node");
}

/// Constants: various literal types in the AST.
#[test]
fn tasty_constants() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Constants.tasty");
    // Name table should contain the constant field names
    assert!(output.contains("hello tasty"), "should contain string constant");
    // AST should contain various constant tags
    assert!(output.contains("BYTEconst"), "should have BYTEconst");
    assert!(output.contains("INTconst"), "should have INTconst");
    assert!(output.contains("LONGconst"), "should have LONGconst");
    assert!(output.contains("DOUBLEconst"), "should have DOUBLEconst");
    assert!(output.contains("STRINGconst"), "should have STRINGconst");
}

/// Simple class with val, var, methods.
#[test]
fn tasty_simple_class() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "SimpleClass.tasty");
    assert!(output.contains("SimpleClass"));
    assert!(output.contains("DEFDEF"), "should have DEFDEF for methods");
    assert!(output.contains("MUTABLE"), "should have MUTABLE for var age");
    assert!(output.contains("greet"));
    assert!(output.contains("birthday"));
}

/// Case class produces CASE modifier.
#[test]
fn tasty_case_class() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Point.tasty");
    assert!(output.contains("CASE"), "case class should have CASE modifier");
    assert!(output.contains("distanceTo"));
}

/// Trait produces TRAIT modifier.
#[test]
fn tasty_trait() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Showable.tasty");
    assert!(output.contains("TRAIT"), "should have TRAIT modifier");
    assert!(output.contains("show"));
}

/// Sealed trait hierarchy.
#[test]
fn tasty_sealed_hierarchy() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Shape.tasty");
    assert!(output.contains("SEALED"), "should have SEALED modifier");
    assert!(output.contains("TRAIT"), "should have TRAIT modifier");
    assert!(output.contains("area"));
    assert!(output.contains("perimeter"));

    let circle = run_tasty(&work_dir, "Circle.tasty");
    assert!(circle.contains("FINAL"), "case class should be FINAL");
    assert!(circle.contains("CASE"));
}

/// Enum produces ENUM modifier.
#[test]
fn tasty_enum() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Direction.tasty");
    assert!(output.contains("ENUM"), "should have ENUM modifier");
    assert!(output.contains("SEALED"), "enum should be SEALED");

    let expr = run_tasty(&work_dir, "Expr.tasty");
    assert!(expr.contains("ENUM"));
}

/// Generic class with covariant/contravariant type params.
#[test]
fn tasty_generics() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Box.tasty");
    assert!(output.contains("TYPEPARAM"), "should have type parameters");
    assert!(output.contains("COVARIANT"), "Box[+A] should have COVARIANT");
    assert!(output.contains("map"));
    assert!(output.contains("flatMap"));

    let converter = run_tasty(&work_dir, "Converter.tasty");
    assert!(
        converter.contains("CONTRAVARIANT"),
        "Converter[-A,+B] should have CONTRAVARIANT"
    );
}

/// Extension methods produce EXTENSION modifier.
#[test]
fn tasty_extensions() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Extensions.tasty");
    assert!(output.contains("EXTENSION"), "should have EXTENSION modifier");
    assert!(output.contains("words"));
    assert!(output.contains("capitalize"));
}

/// Given instances produce GIVEN modifier.
#[test]
fn tasty_givens() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Givens.tasty");
    assert!(output.contains("GIVEN"), "should have GIVEN modifier");
    assert!(output.contains("intOrdering"));
}

/// Inline methods produce INLINE modifier.
#[test]
fn tasty_inline() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "InlineMethod.tasty");
    assert!(output.contains("INLINE"), "should have INLINE modifier");
    assert!(output.contains("assertPositive"));
}

/// Pattern matching produces MATCH and CASEDEF nodes.
#[test]
fn tasty_pattern_match() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "PatternMatch.tasty");
    assert!(output.contains("MATCH"), "should have MATCH node");
    assert!(output.contains("CASEDEF"), "should have CASEDEF node");
    assert!(output.contains("BIND"), "should have BIND node for pattern vars");
    assert!(output.contains("describe"));
}

/// Higher-kinded types produce TYPELAMBDAtype.
#[test]
fn tasty_higher_kinded() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Functor.tasty");
    assert!(
        output.contains("TYPELAMBDAtype"),
        "F[_] should produce TYPELAMBDAtype"
    );
    assert!(output.contains("TRAIT"));
}

/// Type aliases and opaque types.
#[test]
fn tasty_type_aliases() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "TypeAliases.tasty");
    assert!(output.contains("OPAQUE"), "should have OPAQUE for opaque types");
    assert!(output.contains("Email"));
    assert!(output.contains("NonNegInt"));
}

/// Annotations produce ANNOTATION nodes.
#[test]
fn tasty_annotations() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Annotations.tasty");
    assert!(output.contains("ANNOTATION"), "should have ANNOTATION node");
    assert!(output.contains("deprecated") || output.contains("Deprecated"));
}

/// Lambdas produce LAMBDA nodes.
#[test]
fn tasty_lambdas() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Lambdas.tasty");
    assert!(output.contains("LAMBDA"), "should have LAMBDA node");
    assert!(output.contains("compose"));
    assert!(output.contains("applyTwice"));
}

/// Try/catch produces TRY and CASEDEF nodes.
#[test]
fn tasty_try_catch() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "TryCatch.tasty");
    assert!(output.contains("TRY"), "should have TRY node");
    assert!(output.contains("CASEDEF"), "catch should have CASEDEF");
    assert!(output.contains("THROW"), "should have THROW node");
}

/// While loops produce WHILE nodes.
#[test]
fn tasty_while_loops() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "WhileLoops.tasty");
    assert!(output.contains("WHILE"), "should have WHILE node");
    assert!(output.contains("gcd"));
    assert!(output.contains("fibonacci"));
}

/// Lazy vals produce LAZY modifier.
#[test]
fn tasty_lazy_val() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "LazyVal.tasty");
    assert!(output.contains("LAZY"), "should have LAZY modifier");
    assert!(output.contains("expensive"));
}

/// Nested objects produce nested TYPEDEF.
#[test]
fn tasty_nested() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Outer.tasty");
    assert!(output.contains("OBJECT"), "should have OBJECT modifier");
    assert!(output.contains("Middle"));
    assert!(output.contains("Inner"));
}

/// Method overloading produces multiple DEFDEF with same name.
#[test]
fn tasty_overloading() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Overloading.tasty");
    // The name table should have multiple SIGNED entries for "process"
    assert!(output.contains("process"));
    // Multiple DEFDEF nodes
    let defdef_count = output.matches("DEFDEF").count();
    assert!(
        defdef_count >= 5,
        "expected at least 5 DEFDEFs for overloaded methods, got {}",
        defdef_count
    );
}

/// Default parameters produce HASDEFAULT modifier and $default$ getters in name table.
#[test]
fn tasty_default_params() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "DefaultParams.tasty");
    assert!(
        output.contains("HASDEFAULT"),
        "should have HASDEFAULT modifier"
    );
    assert!(output.contains("$default$"), "should have $default$ names");
}

/// Union and intersection types produce ORtype and ANDtype.
#[test]
fn tasty_union_intersection() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "UnionIntersection.tasty");
    // Scala 3 represents union/intersection as applied types with | and & constructors
    assert!(output.contains("[=|]"), "union type should reference | type");
    assert!(output.contains("[=&]"), "intersection type should reference & type");
}

/// Context functions file parses successfully and contains expected names.
#[test]
fn tasty_context_functions() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "ContextFunctions.tasty");
    assert!(output.contains("Logger"));
    assert!(output.contains("withLogging"));
    assert!(output.contains("compute"));
}

/// Monad file with complex higher-kinded given instances.
#[test]
fn tasty_monad() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Monad.tasty");
    assert!(output.contains("TRAIT"));
    assert!(output.contains("pure"));
    assert!(output.contains("flatMap"));
}

/// Attributes section: check SOURCEFILEattr is parsed correctly.
#[test]
fn tasty_source_file_attr() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Empty.tasty");
    assert!(
        output.contains("SOURCEFILEattr"),
        "should have SOURCEFILEattr"
    );
    assert!(
        output.contains("Empty.scala"),
        "SOURCEFILEattr should reference source file"
    );
}

/// All files have valid UUIDs (32 hex chars).
#[test]
fn tasty_valid_uuid() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Constants.tasty");
    for line in output.lines() {
        if line.trim_start().starts_with("uuid: ") {
            let uuid = line.trim_start().strip_prefix("uuid: ").unwrap();
            assert_eq!(uuid.len(), 32, "UUID should be 32 hex chars");
            assert!(
                uuid.chars().all(|c| c.is_ascii_hexdigit()),
                "UUID should be hex: {}",
                uuid
            );
        }
    }
}

/// Positions section is parsed.
#[test]
fn tasty_positions_section() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "SimpleClass.tasty");
    assert!(
        output.contains("Positions ("),
        "should parse Positions section"
    );
}

/// Name table reconstruction: qualified names use dot notation.
#[test]
fn tasty_qualified_names() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Empty.tasty");
    assert!(
        output.contains("java.lang"),
        "qualified name should show as java.lang"
    );
    assert!(
        output.contains("java.lang.Object"),
        "deeply qualified names should reconstruct"
    );
}

/// Companion objects generate ObjectClass name entries (name$).
#[test]
fn tasty_object_class_names() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Empty.tasty");
    assert!(
        output.contains("Empty$"),
        "object class name should show as Empty$"
    );
}

/// Generic case class Pair has type parameters.
#[test]
fn tasty_generic_case_class() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Pair.tasty");
    assert!(output.contains("TYPEPARAM"), "Pair[A,B] should have TYPEPARAMs");
    assert!(output.contains("CASE"));
}

/// Binder type parameters (TYPELAMBDAtype) are parsed correctly with param names.
#[test]
fn tasty_method_type_binders() {
    let work_dir = get_work_dir();

    let output = run_tasty(&work_dir, "Givens.tasty");
    assert!(
        output.contains("TYPELAMBDAtype"),
        "polymorphic givens should produce TYPELAMBDAtype"
    );
    // Should have binder param names
    assert!(output.contains("param "), "binder types should show param names");
}
