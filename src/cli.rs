use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rGradle")]
#[command(about = "An experimental build tool for Java written in Rust", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new Rustapack project
    Init,

    /// Fetch dependencies from Maven Central
    Fetch,

    /// Build the Java project
    Build,

    /// Clean the build directory
    Clean,

    /// Run the Java project (main class must be configured)
    Run,

    /// Run the project tests
    Test,

    /// Package the application into a JAR
    Package {
        #[arg(long)]
        uber: bool,
    },
}
