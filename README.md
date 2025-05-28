# rrrGradle

An experimental build tool for Java written in Rust.

## Installation

To install rrrGradle, you can clone the repository and build the project using Cargo:

```
git clone https://github.com/your-username/rrrGradle.git
cd rrrGradle
cargo build --release
```

The compiled binary will be located in the `target/release` directory.

## Usage

rrrGradle provides the following commands:

- `init`: Initialize a new Rustapack project.
- `fetch`: Fetch dependencies from Maven Central.
- `build`: Build the Java project.
- `clean`: Clean the build directory.
- `run`: Run the Java project (main class must be configured).

To use these commands, run the rrrGradle binary with the desired command:

```
./rrrGradle init
./rrrGradle fetch
./rrrGradle build
./rrrGradle clean
./rrrGradle run
```

## API

The rrrGradle project consists of the following main modules:

- `build.rs`: Handles the compilation of Java source files.
- `cli.rs`: Defines the command-line interface using Clap.
- `config.rs`: Loads the project configuration from the `rrrGradle.toml` file.
- `fetch.rs`: Fetches dependencies from Maven Central.
- `pom.rs`: Parses Maven POM files.
- `run.rs`: Runs the Java application.

## Contributing

Contributions to the rrrGradle project are welcome. To contribute, please follow these steps:

1. Fork the repository.
2. Create a new branch for your feature or bug fix.
3. Make your changes and commit them.
4. Push your changes to your forked repository.
5. Submit a pull request to the main repository.

## License

The rrrGradle project is licensed under the [Apache 2.0 License](LICENSE).

## Testing

To run the tests for the rrrGradle project, use the following command:

```
cargo test
```

This will execute all the unit tests for the project.