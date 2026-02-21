# sb - Ultra-fast Scala 3 Build Tool

## What is this?

A minimal, cargo/uv-style CLI build tool for Scala 3, written in Rust. No interactive console — just `sb init`, `sb build`, `sb run`, `sb clean`.

## Architecture

```
src/
  main.rs      — CLI entry point (clap). Command dispatch for init/build/run/clean.
  config.rs    — sb.toml parsing. SbConfig/Project structs via serde.
  cache.rs     — xxh3 fingerprinting of sources and deps. Cache read/write to .sb/cache/.
  resolve.rs   — Coursier (cs) wrapper. Resolves compiler + user classpaths. Parallel resolution with indicatif spinners.
  compile.rs   — Orchestrates compilation. Parallel resolve + source hashing via std::thread::scope. Invokes dotc via java subprocess.
  run.rs       — Compiles then exec's java with the main class (Unix exec replaces process).
```

## Key design decisions

- **No JVM for the tool itself** — Rust binary starts in <1ms
- **Coursier (`cs`) as subprocess** — dependency resolution. Classpath cached in `.sb/cache/classpath`, keyed by dep-hash
- **Compiler invocation** — `java -cp <compiler_cp> dotty.tools.dotc.Main -classpath <user_cp> -d .sb/classes <sources>`
- **Full recompilation** — no incremental/Zinc. All sources passed to one dotc invocation. No-op builds skip via hash comparison (~4ms)
- **Scala cross-version** — `org::name:version` rewritten to `org:name_3:version` (hardcoded to `_3`)

## sb.toml format

```toml
[project]
name = "myapp"
version = "0.1.0"
scala-version = "3.6.4"
main-class = "hello"           # optional, @main method name
dependencies = [
  "org.typelevel::cats-core:2.12.0",   # :: = Scala dep
  "com.google.guava:guava:33.0.0-jre", # :  = Java dep
]
scalac_options = ["-Werror"]   # optional compiler flags
```

## Project layout

```
sb.toml
src/main/scala/   # Scala source files (recursive)
.sb/              # build artifacts (gitignored)
  cache/          # dep-hash, classpath, src-hash
  classes/        # compiled .class and .tasty files
```

## Testing

Integration tests in `tests/integration.rs`. Test projects auto-discovered:

- `tests/pos/` — projects that must compile and run successfully
- `tests/neg/` — projects that must fail compilation

Run: `cargo test`

To add a test: create a directory with `sb.toml` + `src/main/scala/` under `tests/pos/` or `tests/neg/`.

## Build & run

```sh
cargo build              # dev build
cargo build --release     # optimized build
cargo test                # run all tests (requires cs + java on PATH)
```

## Prerequisites

- `cs` (Coursier) on PATH — for dependency resolution
- `java` on PATH — for running scalac and compiled programs
- JDK 21+ recommended (older Scala 3 versions may not work with JDK 25+)

## Current limitations

- No incremental compilation (full recompile on any source change)
- No test framework support (`sb test`)
- No multi-module projects
- No watch mode
- TASTy-based dependency analysis planned but not implemented
