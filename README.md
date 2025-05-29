# 🚀 rrrGradle

An experimental build tool for Java written in Rust to achieve blazingly fast modern build tool for Java projects powered by Rust. rrrGradle combines the simplicity of Gradle with the performance of Rust to deliver an exceptional Java build experience.

## ✨ Key Features

- **Lightning Fast Builds** - Built with Rust for maximum performance
- **Smart Incremental Compilation** - Only rebuilds what's necessary
- **Parallel Dependency Resolution** - Concurrent Maven artifact downloads
- **Simple TOML Configuration** - Intuitive project setup using TOML instead of Groovy/XML
- **Zero Dependencies** - No JVM required for the build tool itself
- **Modern Project Templates** - Instant project setup with best practices
- **Integrated Test Runner** - JUnit test execution built-in
- **Uber JAR Support** - Package your app with all dependencies

## 🔧 Installation

rrrGradle is distributed as a single binary with zero dependencies. To install:

```
git clone https://github.com/your-username/rrrGradle.git
cd rrrGradle
cargo build --release
```

The compiled binary will be located in the `target/release` directory.

## 📖 Quick Start

Create a new Java project in seconds:

```powershell
rrrGradle init    # Creates a new project with example code
rrrGradle fetch   # Downloads dependencies
rrrGradle build   # Compiles your code
rrrGradle run     # Runs your application
```

## 🛠️ Command Reference

rrrGradle offers an intuitive command set for your Java development workflow:

- `init` - Bootstrap a new project with:
  - Standard Maven-style directory structure
  - Sample main class with "Hello World"
  - JUnit test setup
  - TOML configuration
- `fetch` - Smart dependency management:
  - Parallel downloads from Maven Central
  - Automatic transitive dependency resolution
  - Local dependency caching
  - Progress bars with download status
- `build` - Efficient compilation:
  - Incremental builds - only recompiles changed files
  - Parallel compilation for faster builds
  - Automatic handling of source and resource files
- `run` - Easy execution:
  - Automatic classpath configuration
  - Support for command-line arguments
  - Configurable main class
- `test` - Integrated testing:
  - JUnit test runner built-in
  - Parallel test execution
  - Clear test reporting
- `clean` - Clean workspace:
  - Removes build artifacts
  - Keeps dependency cache
- `package` - Create distributable JARs:
  - Regular JAR with manifest
  - Uber/Fat JAR with all dependencies

To use these commands, run the rrrGradle binary with the desired command:

```
./rrrGradle init
./rrrGradle fetch
./rrrGradle build
./rrrGradle clean
./rrrGradle run
```

## 📝 Project Configuration

rrrGradle uses TOML for configuration, making it clear and easy to maintain:

```toml
[project]
name = "MyAwesomeApp"
version = "1.0.0"
main_class = "com.example.Main"

[dependencies]
"com.google.guava:guava" = "31.1-jre"
"org.slf4j:slf4j-api" = "2.0.9"

[test_dependencies]
"junit:junit" = "4.13.2"
```

## 🏗️ Project Structure

rrrGradle follows Maven-style project conventions:

```
your-project/
├── src/
│   ├── main/
│   │   ├── java/         # Java source files
│   │   └── resources/    # Resource files
│   └── test/
│       ├── java/         # Test source files
│       └── resources/    # Test resources
├── build/
│   └── classes/          # Compiled classes
├── target/
│   └── myapp.jar        # Generated JAR
└── rrrgradle.toml       # Project configuration
```

## ⚡ Performance

rrrGradle is designed for speed:

- **Rust-Powered Core**: Built with Rust for native performance
- **Smart Caching**: Efficient caching of dependencies and build artifacts
- **Parallel Processing**: Utilizes all CPU cores for builds and downloads
- **Minimal Overhead**: No JVM startup time for the build tool

## 🤝 Contributing

We love your input! rrrGradle is looking for contributors. Here's how you can help:

- 🐛 Report bugs and issues
- 💡 Propose new features
- 📖 Improve documentation
- 💻 Submit pull requests

### Development Setup

1. Clone the repository:

```powershell
git clone https://github.com/your-username/rrrGradle.git
cd rrrGradle
```

2. Build the project:

```powershell
cargo build
```

3. Run tests:

```powershell
cargo test
```

## 📋 Roadmap

Future enhancements planned:

- [ ] Multiple JDK version support
- [ ] Custom plugin system
- [ ] Native library support
- [ ] IDE integration
- [ ] Docker container support
- [ ] GitHub Actions integration
- [ ] Dependency vulnerability scanning

## ⭐ Why rrrGradle?

- **Fast**: Native performance with Rust
- **Simple**: Clear TOML configuration
- **Modern**: Built for today's Java development
- **Efficient**: Smart incremental builds
- **Lightweight**: Single binary, no runtime dependencies
- **Productive**: Integrated testing and packaging

## 📄 License

rrrGradle is open source software licensed under the [Apache 2.0 License](LICENSE).
