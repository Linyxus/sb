# sb

Ultra-fast, minimalist build tool for Scala 3.

No console. No XML. No waiting. Just build.

## Install

```sh
cargo install --path .
```

Requires `java` and [`cs` (Coursier)](https://get-coursier.io/docs/cli-installation) on your PATH.

## Quickstart

```sh
sb init myapp
cd myapp
sb run
```

## Commands

| Command | Description |
|---------|-------------|
| `sb init <name>` | Create a new project (use `.` for current directory) |
| `sb build` | Compile the project |
| `sb run [args...]` | Compile and run |
| `sb clean` | Remove build artifacts |

## Configuration

Projects are configured with a single `sb.toml`:

```toml
[project]
name = "myapp"
version = "0.1.0"
scala-version = "3.6.4"
main-class = "hello"
dependencies = [
  "org.typelevel::cats-core:2.12.0",
  "com.lihaoyi::os-lib:0.11.4",
]
scalac_options = ["-Werror"]
```

- `::` for Scala dependencies (cross-versioned)
- `:` for Java dependencies

## Project layout

```
myapp/
├── sb.toml
└── src/
    └── main/
        └── scala/
            └── Main.scala
```

## Performance

sb is designed to be fast:

- **Native binary** — no JVM startup for the tool itself
- **Cached dependency resolution** — Coursier only runs when dependencies change
- **Source fingerprinting** — no-op builds complete in ~4ms
- **Parallel I/O** — dependency resolution and source hashing run concurrently

## License

MIT
