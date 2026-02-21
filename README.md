# sb

Ultra-fast, minimalist build tool for Scala 3.

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
| `sb asm` | Assemble a fat JAR |
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

## `sb.toml` Reference

The project configuration file has a single `[project]` table with the following fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Project name |
| `version` | string | yes | Project version |
| `scala-version` | string | yes | Scala 3 compiler version (e.g. `"3.6.4"`) |
| `main-class` | string | no | Name of the `@main` method to run with `sb run` |
| `dependencies` | array of strings | no | Library dependencies (default: `[]`) |
| `scalac_options` | array of strings | no | Extra flags passed to the Scala compiler (default: `[]`) |

### Dependency syntax

Dependencies are specified as Maven coordinates with a colon-separated format:

- **Scala dependency** (`::`) — `"org::artifact:version"` is expanded to `org:artifact_3:version`, appending the `_3` cross-version suffix automatically.
- **Java dependency** (`:`) — `"org:artifact:version"` is used as-is, with no cross-version rewriting.

### Full example

```toml
[project]
name = "myapp"
version = "0.1.0"
scala-version = "3.6.4"
main-class = "hello"
dependencies = [
  "org.typelevel::cats-core:2.12.0",
  "com.lihaoyi::os-lib:0.11.4",
  "com.google.guava:guava:33.0.0-jre",
]
scalac_options = ["-Werror", "-explain"]
```

## License

MIT
