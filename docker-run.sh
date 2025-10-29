#!/bin/bash
# Docker deployment script for LlamaPackageService
# Author: Nik Jois <nikjois@llamasearch.ai>

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}$1${NC}"
}

# Function to check if Docker is running
check_docker() {
    if ! docker info >/dev/null 2>&1; then
        print_error "Docker is not running. Please start Docker and try again."
        exit 1
    fi
}

# Function to check if docker-compose is available
check_docker_compose() {
    if ! command -v docker-compose >/dev/null 2>&1; then
        print_error "docker-compose is not installed. Please install it and try again."
        exit 1
    fi
}

# Function to create necessary directories
create_directories() {
    print_status "Creating necessary directories..."
    mkdir -p output logs data input monitoring/grafana/provisioning
    chmod 755 output logs data input
}

# Function to check environment variables
check_env_vars() {
    print_status "Checking environment variables..."
    
    if [ -z "$GITHUB_TOKEN" ]; then
        print_warning "GITHUB_TOKEN not set. GitHub API functionality will be limited."
    fi
    
    if [ -z "$PYPI_API_KEY" ]; then
        print_warning "PYPI_API_KEY not set. PyPI functionality will be limited."
    fi
    
    if [ -z "$NPM_API_KEY" ]; then
        print_warning "NPM_API_KEY not set. NPM functionality will be limited."
    fi
    
    if [ -z "$MLX_API_KEY" ]; then
        print_warning "MLX_API_KEY not set. MLX integration will be disabled."
    fi
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  build       Build the Docker image"
    echo "  server      Start the web server"
    echo "  cli         Run CLI mode"
    echo "  dev         Start in development mode with monitoring"
    echo "  prod        Start in production mode"
    echo "  stop        Stop all services"
    echo "  clean       Clean up containers and images"
    echo "  logs        Show logs"
    echo "  health      Check service health"
    echo ""
    echo "Options:"
    echo "  --cache     Include Redis cache"
    echo "  --monitor   Include monitoring stack"
    echo "  --rebuild   Force rebuild of images"
    echo ""
    echo "Examples:"
    echo "  $0 build --rebuild"
    echo "  $0 server --cache"
    echo "  $0 dev --monitor"
    echo "  $0 cli"
    echo ""
}

# Function to build Docker image
build_image() {
    local rebuild=${1:-false}
    
    print_header "Building LlamaPackageService Docker Image"
    
    if [ "$rebuild" = true ]; then
        print_status "Force rebuilding image..."
        docker-compose build --no-cache
    else
        print_status "Building image..."
        docker-compose build
    fi
    
    print_status "Build completed successfully!"
}

# Function to start server
start_server() {
    local cache=${1:-false}
    local monitor=${2:-false}
    
    print_header "Starting LlamaPackageService Server"
    
    local profiles=""
    
    if [ "$cache" = true ]; then
        profiles="$profiles --profile cache"
        print_status "Including Redis cache..."
    fi
    
    if [ "$monitor" = true ]; then
        profiles="$profiles --profile monitoring"
        print_status "Including monitoring stack..."
    fi
    
    print_status "Starting services..."
    docker-compose up -d llamapackageservice-server $profiles
    
    print_status "Server starting... checking health..."
    sleep 5
    
    # Wait for service to be healthy
    for i in {1..30}; do
        if curl -f http://localhost:8080/health >/dev/null 2>&1; then
            print_status "Service is healthy and ready!"
            echo ""
            print_status "Web interface: http://localhost:8080"
            if [ "$monitor" = true ]; then
                print_status "Grafana dashboard: http://localhost:3000 (admin/admin)"
                print_status "Prometheus metrics: http://localhost:9090"
            fi
            return 0
        fi
        sleep 2
    done
    
    print_error "Service failed to start properly. Check logs with: $0 logs"
    return 1
}

# Function to run CLI
run_cli() {
    print_header "Running LlamaPackageService CLI"
    
    print_status "Starting CLI container..."
    docker-compose run --rm llamapackageservice-cli llamapackageservice "$@"
}

# Function to start development environment
start_dev() {
    print_header "Starting Development Environment"
    
    print_status "Starting all services with monitoring..."
    docker-compose --profile cache --profile monitoring up -d
    
    print_status "Development environment ready!"
    echo ""
    print_status "Services:"
    print_status "  - Web Server: http://localhost:8080"
    print_status "  - Grafana: http://localhost:3000 (admin/admin)"
    print_status "  - Prometheus: http://localhost:9090"
    print_status "  - Redis: localhost:6379"
}

# Function to start production environment
start_prod() {
    print_header "Starting Production Environment"
    
    print_status "Starting production services..."
    docker-compose --profile cache up -d llamapackageservice-server redis
    
    print_status "Production environment ready!"
    print_status "Web Server: http://localhost:8080"
}

# Function to stop services
stop_services() {
    print_header "Stopping LlamaPackageService"
    
    print_status "Stopping all services..."
    docker-compose --profile cache --profile monitoring --profile cli down
    
    print_status "All services stopped."
}

# Function to clean up
cleanup() {
    print_header "Cleaning Up LlamaPackageService"
    
    print_status "Stopping and removing containers..."
    docker-compose --profile cache --profile monitoring --profile cli down --remove-orphans
    
    print_status "Removing images..."
    docker rmi $(docker images llamapackageservice -q) 2>/dev/null || true
    
    print_status "Cleaning up dangling images..."
    docker image prune -f
    
    print_status "Cleanup completed."
}

# Function to show logs
show_logs() {
    print_header "LlamaPackageService Logs"
    
    docker-compose logs -f --tail=100 "$@"
}

# Function to check health
check_health() {
    print_header "Health Check"
    
    # Check if containers are running
    if docker-compose ps | grep -q "Up"; then
        print_status "Containers are running"
        
        # Check web service health
        if curl -f http://localhost:8080/health >/dev/null 2>&1; then
            print_status "Web service is healthy"
        else
            print_error "Web service is not responding"
        fi
        
        # Check Redis if running
        if docker-compose ps redis 2>/dev/null | grep -q "Up"; then
            if docker-compose exec redis redis-cli ping >/dev/null 2>&1; then
                print_status "Redis is healthy"
            else
                print_error "Redis is not responding"
            fi
        fi
    else
        print_error "No containers are running"
    fi
}

# Main script logic
main() {
    # Prerequisites
    check_docker
    check_docker_compose
    create_directories
    check_env_vars
    
    local command=$1
    shift || true
    
    # Parse options
    local cache=false
    local monitor=false
    local rebuild=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --cache)
                cache=true
                shift
                ;;
            --monitor)
                monitor=true
                shift
                ;;
            --rebuild)
                rebuild=true
                shift
                ;;
            *)
                break
                ;;
        esac
    done
    
    case $command in
        build)
            build_image $rebuild
            ;;
        server)
            build_image
            start_server $cache $monitor
            ;;
        cli)
            build_image
            run_cli "$@"
            ;;
        dev)
            build_image
            start_dev
            ;;
        prod)
            build_image
            start_prod
            ;;
        stop)
            stop_services
            ;;
        clean)
            cleanup
            ;;
        logs)
            show_logs "$@"
            ;;
        health)
            check_health
            ;;
        *)
            show_usage
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@" 