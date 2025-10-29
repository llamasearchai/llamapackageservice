# Contributing to LlamaPackageService

Thank you for your interest in contributing to LlamaPackageService! This document provides guidelines and instructions for contributing to this project.

## Code of Conduct

This project adheres to a Code of Conduct that all contributors are expected to follow. By participating, you are expected to uphold this code. Please report unacceptable behavior.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the issue tracker to see if the problem has already been reported. When you are creating a bug report, please include as many details as possible:

* **Use a clear and descriptive title**
* **Describe the exact steps to reproduce the bug**
* **Provide specific examples** to demonstrate the steps
* **Describe the behavior you observed after following the steps**
* **Explain which behavior you expected to see instead and why**
* **Include screenshots or animated GIFs** if possible
* **Include the version of the application** you're using
* **Include your operating system and version**

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When you are creating an enhancement suggestion, please include:

* **Use a clear and descriptive title**
* **Provide a step-by-step description of the suggested enhancement**
* **Provide specific examples to demonstrate the steps**
* **Describe the current behavior and explain which behavior you expected to see instead**
* **Explain why this enhancement would be useful**
* **List similar implementations of this feature in other projects if you know of any**

### Your First Code Contribution

Unsure where to begin contributing? You can start by looking through these `beginner` and `help-wanted` issues:

* **Beginner issues** - issues which should only require a few lines of code
* **Help wanted issues** - issues which should be a bit more involved than beginner issues

### Pull Requests

* Fill in the required template
* Follow the Rust style guide
* Include tests when adding new features
* Update documentation for API changes
* Ensure the test suite passes
* Make sure your code lints without errors

## Development Process

### Setup Development Environment

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/llamapackageservice.git`
3. Create a branch for your feature: `git checkout -b feature/amazing-feature`
4. Install development dependencies: `cargo check`

### Code Style

* Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
* Run `cargo fmt` before committing
* Run `cargo clippy` to catch common mistakes
* Document all public API elements with appropriate doc comments

### Testing

* Write tests for all new features
* Ensure all tests pass: `cargo test`
* Include integration tests when appropriate
* Check code coverage for your changes

### Documentation

* Update the README.md with details of changes to the interface
* Update the API documentation for any modified functionality
* Add examples for new features when appropriate

### Commit Messages

* Use the present tense ("Add feature" not "Added feature")
* Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
* Limit the first line to 72 characters or less
* Reference issues and pull requests liberally after the first line
* Consider using conventional commits format:
  * `feat: add new feature X`
  * `fix: resolve issue with Y`
  * `docs: update API documentation`
  * `test: add tests for feature Z`
  * `refactor: improve code structure`

## Release Process

### Versioning

This project follows [Semantic Versioning](https://semver.org/). In short:

* MAJOR version for incompatible API changes
* MINOR version for new functionality in a backwards compatible manner
* PATCH version for backwards compatible bug fixes

### Creating a Release

1. Update the version in Cargo.toml
2. Update CHANGELOG.md with the changes
3. Commit these changes: `git commit -m "chore: release version X.Y.Z"`
4. Tag the commit: `git tag vX.Y.Z`
5. Push the commit and tag: `git push && git push --tags`

## Additional Resources

* [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
* [How to Write a Git Commit Message](https://chris.beams.io/posts/git-commit/)
* [GitHub Pull Request Guide](https://docs.github.com/en/github/collaborating-with-issues-and-pull-requests/about-pull-requests)

Thank you for contributing to LlamaPackageService! 