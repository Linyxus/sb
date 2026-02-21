mod cache;
mod compile;
mod config;
mod maven;
mod resolve;
mod run;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sb", about = "Ultra-fast Scala 3 build tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Scala 3 project
    Init {
        /// Project name, or "." to init in current directory
        name: String,
    },
    /// Compile the project
    Build,
    /// Compile and run the main class
    Run {
        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Remove build artifacts
    Clean,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => cmd_init(name),
        Commands::Build => cmd_build(),
        Commands::Run { args } => cmd_run(&args),
        Commands::Clean => cmd_clean(),
    }
}

fn project_root() -> Result<PathBuf> {
    Ok(std::env::current_dir()?)
}

fn cmd_init(name: String) -> Result<()> {
    let cwd = project_root()?;
    let (root, name) = if name == "." {
        let dir_name = cwd
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "myapp".to_string());
        (cwd, dir_name)
    } else {
        let dir = cwd.join(&name);
        std::fs::create_dir_all(&dir)?;
        (dir, name)
    };

    let toml_path = root.join("sb.toml");
    if toml_path.exists() {
        anyhow::bail!("sb.toml already exists in {}", root.display());
    }

    std::fs::write(
        &toml_path,
        format!(
            r#"[project]
name = "{name}"
version = "0.1.0"
scala-version = "3.6.4"
main-class = "hello"
dependencies = []
"#
        ),
    )?;

    let src_dir = root.join("src/main/scala");
    std::fs::create_dir_all(&src_dir)?;
    std::fs::write(
        src_dir.join("Main.scala"),
        r#"@main def hello(): Unit =
  println("Hello from sb!")
"#,
    )?;

    eprintln!("Initialized project '{name}'");
    Ok(())
}

fn cmd_build() -> Result<()> {
    let root = project_root()?;
    let config = config::SbConfig::load(&root)?;
    compile::compile(&config, &root)?;
    Ok(())
}

fn cmd_run(args: &[String]) -> Result<()> {
    let root = project_root()?;
    let config = config::SbConfig::load(&root)?;
    run::run(&config, &root, args)
}

fn cmd_clean() -> Result<()> {
    let root = project_root()?;
    let sb_dir = root.join(".sb");
    if sb_dir.exists() {
        std::fs::remove_dir_all(&sb_dir)?;
        eprintln!("Cleaned .sb/");
    }
    Ok(())
}
