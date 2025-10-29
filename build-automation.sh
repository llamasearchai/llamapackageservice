#!/bin/bash
# Comprehensive Build Automation Script for LlamaPackageService
# Author: Nik Jois <nikjois@llamasearch.ai>

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
RUST_VERSION="1.75"
CARGO_PROFILE=${CARGO_PROFILE:-release}
TEST_TIMEOUT=${TEST_TIMEOUT:-300}
COVERAGE_THRESHOLD=${COVERAGE_THRESHOLD:-80}

# Function to print colored output
print_status() {
    echo -e "${GREEN}[BUILD]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}======================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}======================================${NC}"
}

# Function to check prerequisites
check_prerequisites() {
    print_header "Checking Prerequisites"
    
    # Check Rust version
    if ! command -v rustc >/dev/null 2>&1; then
        print_error "Rust is not installed. Please install Rust $RUST_VERSION or later."
        exit 1
    fi
    
    local rust_version=$(rustc --version | cut -d' ' -f2)
    print_status "Rust version: $rust_version"
    
    # Check Cargo
    if ! command -v cargo >/dev/null 2>&1; then
        print_error "Cargo is not available."
        exit 1
    fi
    
    # Install additional tools if needed
    print_status "Installing additional tools..."
    
    # Code coverage
    if ! cargo --list | grep -q "tarpaulin"; then
        print_status "Installing cargo-tarpaulin for code coverage..."
        cargo install cargo-tarpaulin
    fi
    
    # Security audit
    if ! cargo --list | grep -q "audit"; then
        print_status "Installing cargo-audit for security auditing..."
        cargo install cargo-audit
    fi
    
    # Benchmarking
    if ! cargo --list | grep -q "criterion"; then
        print_status "Installing cargo-criterion for benchmarking..."
        cargo install cargo-criterion
    fi
    
    # Linting
    if ! rustup component list --installed | grep -q "clippy"; then
        print_status "Installing clippy for linting..."
        rustup component add clippy
    fi
    
    # Formatting
    if ! rustup component list --installed | grep -q "rustfmt"; then
        print_status "Installing rustfmt for code formatting..."
        rustup component add rustfmt
    fi
    
    print_status "All prerequisites satisfied!"
}

# Function to clean build artifacts
clean_build() {
    print_header "Cleaning Build Artifacts"
    
    print_status "Cleaning Cargo build cache..."
    cargo clean
    
    print_status "Removing output directories..."
    rm -rf output/ logs/ coverage/ benchmarks/
    
    print_status "Creating fresh directories..."
    mkdir -p output logs coverage benchmarks
    
    print_status "Clean completed!"
}

# Function to format code
format_code() {
    print_header "Code Formatting"
    
    print_status "Running rustfmt..."
    cargo fmt -- --check || {
        print_warning "Code formatting issues found. Auto-fixing..."
        cargo fmt
        print_status "Code formatted successfully!"
    }
}

# Function to run linting
run_linting() {
    print_header "Code Linting"
    
    print_status "Running clippy lints..."
    cargo clippy --all-targets --all-features -- -D warnings
    
    print_status "Linting completed successfully!"
}

# Function to run security audit
security_audit() {
    print_header "Security Audit"
    
    print_status "Updating audit database..."
    cargo audit --db
    
    print_status "Running security audit..."
    cargo audit
    
    print_status "Security audit completed!"
}

# Function to build the project
build_project() {
    print_header "Building Project"
    
    print_status "Building in $CARGO_PROFILE mode..."
    
    if [ "$CARGO_PROFILE" = "release" ]; then
        cargo build --release --all-features
    else
        cargo build --all-features
    fi
    
    print_status "Build completed successfully!"
}

# Function to run tests
run_tests() {
    print_header "Running Tests"
    
    print_status "Running unit tests..."
    RUST_LOG=debug cargo test --lib --timeout $TEST_TIMEOUT
    
    print_status "Running integration tests..."
    RUST_LOG=debug cargo test --test '*' --timeout $TEST_TIMEOUT
    
    print_status "Running documentation tests..."
    cargo test --doc
    
    print_status "All tests passed!"
}

# Function to run code coverage
run_coverage() {
    print_header "Code Coverage Analysis"
    
    print_status "Running coverage analysis with tarpaulin..."
    cargo tarpaulin \
        --out Html \
        --output-dir coverage \
        --timeout $TEST_TIMEOUT \
        --exclude-files 'target/*' \
        --exclude-files 'tests/*' \
        --fail-under $COVERAGE_THRESHOLD
    
    print_status "Coverage report generated in coverage/tarpaulin-report.html"
    
    # Extract coverage percentage
    local coverage=$(grep -o '[0-9.]*%' coverage/tarpaulin-report.html | head -n1 | tr -d '%')
    print_status "Current coverage: ${coverage}%"
    
    if (( $(echo "$coverage >= $COVERAGE_THRESHOLD" | bc -l) )); then
        print_status "Coverage threshold met!"
    else
        print_error "Coverage below threshold of ${COVERAGE_THRESHOLD}%"
        exit 1
    fi
}

# Function to run benchmarks
run_benchmarks() {
    print_header "Performance Benchmarks"
    
    print_status "Running benchmarks..."
    cargo bench --bench '*' -- --output-format json > benchmarks/results.json
    
    print_status "Benchmark results saved to benchmarks/results.json"
}

# Function to generate documentation
generate_docs() {
    print_header "Generating Documentation"
    
    print_status "Building documentation..."
    cargo doc --all-features --no-deps
    
    print_status "Documentation generated in target/doc/"
}

# Function to package the application
package_application() {
    print_header "Packaging Application"
    
    print_status "Creating release package..."
    
    # Create package directory
    local package_dir="llamapackageservice-$(cargo pkgid | cut -d'#' -f2)"
    mkdir -p "$package_dir"
    
    # Copy binaries
    if [ "$CARGO_PROFILE" = "release" ]; then
        cp target/release/llamapackageservice "$package_dir/"
        cp target/release/server "$package_dir/"
    else
        cp target/debug/llamapackageservice "$package_dir/"
        cp target/debug/server "$package_dir/"
    fi
    
    # Copy assets and documentation
    cp -r assets/ "$package_dir/" 2>/dev/null || true
    cp -r templates/ "$package_dir/" 2>/dev/null || true
    cp README.md "$package_dir/" 2>/dev/null || true
    cp LICENSE "$package_dir/" 2>/dev/null || true
    
    # Create archive
    tar -czf "${package_dir}.tar.gz" "$package_dir"
    
    print_status "Package created: ${package_dir}.tar.gz"
    
    # Cleanup
    rm -rf "$package_dir"
}

# Function to deploy to staging
deploy_staging() {
    print_header "Deploying to Staging"
    
    print_status "Building Docker image for staging..."
    ./docker-run.sh build --rebuild
    
    print_status "Starting staging environment..."
    ./docker-run.sh dev
    
    print_status "Running health checks..."
    sleep 10
    ./docker-run.sh health
    
    print_status "Staging deployment completed!"
}

# Function to run performance tests
performance_tests() {
    print_header "Performance Testing"
    
    print_status "Running stress tests..."
    
    # Example performance test
    if command -v ab >/dev/null 2>&1; then
        print_status "Running Apache Bench tests..."
        ab -n 100 -c 10 http://localhost:8080/health > benchmarks/stress-test.log
    else
        print_warning "Apache Bench not available, skipping stress tests"
    fi
    
    print_status "Performance testing completed!"
}

# Function to validate configuration
validate_config() {
    print_header "Configuration Validation"
    
    print_status "Validating Cargo.toml..."
    cargo metadata --format-version 1 > /dev/null
    
    print_status "Validating Docker configuration..."
    docker-compose config > /dev/null
    
    print_status "Configuration validation completed!"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  full        Run complete build pipeline"
    echo "  clean       Clean build artifacts"
    echo "  format      Format code"
    echo "  lint        Run linting checks"
    echo "  audit       Run security audit"
    echo "  build       Build the project"
    echo "  test        Run tests"
    echo "  coverage    Run code coverage analysis"
    echo "  bench       Run performance benchmarks"
    echo "  docs        Generate documentation"
    echo "  package     Package the application"
    echo "  deploy      Deploy to staging"
    echo "  perf        Run performance tests"
    echo "  validate    Validate configuration"
    echo "  quick       Quick build and test"
    echo ""
    echo "Environment Variables:"
    echo "  CARGO_PROFILE      Build profile (debug|release) [default: release]"
    echo "  TEST_TIMEOUT       Test timeout in seconds [default: 300]"
    echo "  COVERAGE_THRESHOLD Coverage threshold percentage [default: 80]"
    echo ""
}

# Function to run full pipeline
run_full_pipeline() {
    print_header "Full Build Pipeline"
    
    local start_time=$(date +%s)
    
    check_prerequisites
    validate_config
    clean_build
    format_code
    run_linting
    security_audit
    build_project
    run_tests
    run_coverage
    run_benchmarks
    generate_docs
    package_application
    
    if [ "${DEPLOY_STAGING:-false}" = "true" ]; then
        deploy_staging
        performance_tests
    fi
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    print_header "Build Pipeline Completed Successfully!"
    print_status "Total time: ${duration} seconds"
    
    # Generate build report
    cat > build-report.txt << EOF
LlamaPackageService Build Report
Generated: $(date)
Duration: ${duration} seconds
Profile: $CARGO_PROFILE
Coverage Threshold: $COVERAGE_THRESHOLD%

Status: SUCCESS
EOF
    
    print_status "Build report saved to build-report.txt"
}

# Function to run quick build
run_quick_build() {
    print_header "Quick Build and Test"
    
    format_code
    run_linting
    build_project
    run_tests
    
    print_status "Quick build completed!"
}

# Main script logic
main() {
    local command=${1:-full}
    
    case $command in
        full)
            run_full_pipeline
            ;;
        clean)
            clean_build
            ;;
        format)
            format_code
            ;;
        lint)
            run_linting
            ;;
        audit)
            security_audit
            ;;
        build)
            build_project
            ;;
        test)
            run_tests
            ;;
        coverage)
            run_coverage
            ;;
        bench)
            run_benchmarks
            ;;
        docs)
            generate_docs
            ;;
        package)
            package_application
            ;;
        deploy)
            deploy_staging
            ;;
        perf)
            performance_tests
            ;;
        validate)
            validate_config
            ;;
        quick)
            run_quick_build
            ;;
        *)
            show_usage
            exit 1
            ;;
    esac
}

# Trap to handle interrupts
trap 'print_error "Build interrupted by user"; exit 130' INT

# Run main function
main "$@" 