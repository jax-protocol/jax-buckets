#!/bin/bash

# Development script - sets up 3 nodes with different blob store backends
#
# Usage:
#   ./bin/dev.sh          # Start all nodes
#   ./bin/dev.sh clean    # Remove all dev data

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m'

S3_URL='s3://minioadmin:minioadmin@localhost:9000/jax-blobs'

clean() {
    echo -e "${YELLOW}Cleaning dev data...${NC}"
    rm -rf ./data/node0 ./data/node1 ./data/node2
    echo -e "${GREEN}Done${NC}"
}

run() {
    echo -e "${BLUE}Setting up JAX dev environment...${NC}"

    # Check cargo-watch
    if ! command -v cargo-watch &>/dev/null; then
        echo -e "${YELLOW}Installing cargo-watch...${NC}"
        cargo install cargo-watch
    fi

    # Start MinIO for node2
    echo -e "${BLUE}Starting MinIO...${NC}"
    "$SCRIPT_DIR/minio.sh" up || true

    # Initialize nodes if needed
    if [ ! -d "./data/node0" ]; then
        echo -e "${YELLOW}Initializing node0 (full, legacy)...${NC}"
        cargo run --bin jax -- --config-path ./data/node0 init \
            --app-port 8080 --peer-port 9000 --gateway-port 9080 \
            --blob-store legacy
    fi

    if [ ! -d "./data/node1" ]; then
        echo -e "${YELLOW}Initializing node1 (app only, filesystem)...${NC}"
        cargo run --bin jax -- --config-path ./data/node1 init \
            --app-port 8081 --peer-port 9001 --gateway-port 9081 \
            --blob-store filesystem
    fi

    if [ ! -d "./data/node2" ]; then
        echo -e "${YELLOW}Initializing node2 (gateway only, s3)...${NC}"
        cargo run --bin jax -- --config-path ./data/node2 init \
            --app-port 8082 --peer-port 9002 --gateway-port 9082 \
            --blob-store s3 --s3-url "$S3_URL"
    fi

    # Kill existing session
    tmux kill-session -t jax-dev 2>/dev/null || true

    # Create tmux session with 3 panes in a single window
    tmux new-session -d -s jax-dev -n nodes

    # Split into 3 vertical panes
    tmux split-window -v -t jax-dev:0
    tmux split-window -v -t jax-dev:0

    # Make panes equal size
    tmux select-layout -t jax-dev:0 even-vertical

    # Node0: full node (app + gateway), legacy
    tmux send-keys -t jax-dev:0.0 "cd $PROJECT_ROOT && echo -e '${GREEN}=== node0: Full Node (legacy) ===${NC}' && echo 'App: http://localhost:8080 | Gateway: http://localhost:9080' && echo '' && RUST_LOG=info cargo watch --why --ignore 'data/*' --ignore '*.sqlite*' --ignore '*.db*' -x 'run --bin jax -- --config-path ./data/node0 daemon --with-gateway'" C-m

    # Node1: app only, filesystem
    tmux send-keys -t jax-dev:0.1 "cd $PROJECT_ROOT && echo -e '${GREEN}=== node1: App Only (filesystem) ===${NC}' && echo 'App: http://localhost:8081' && echo '' && RUST_LOG=info cargo watch --why --ignore 'data/*' --ignore '*.sqlite*' --ignore '*.db*' -x 'run --bin jax -- --config-path ./data/node1 daemon'" C-m

    # Node2: gateway only, s3
    tmux send-keys -t jax-dev:0.2 "cd $PROJECT_ROOT && echo -e '${GREEN}=== node2: Gateway Only (s3/minio) ===${NC}' && echo 'Gateway: http://localhost:9082' && echo '' && RUST_LOG=info cargo watch --why --ignore 'data/*' --ignore '*.sqlite*' --ignore '*.db*' -x 'run --bin jax -- --config-path ./data/node2 daemon --gateway'" C-m

    echo ""
    echo -e "${GREEN}Started 3 nodes:${NC}"
    echo "  node0: http://localhost:8080 (full, legacy)"
    echo "  node1: http://localhost:8081 (app only, filesystem)"
    echo "  node2: http://localhost:9082 (gateway only, s3)"
    echo ""
    echo "MinIO console: http://localhost:9001"
    echo ""

    tmux attach -t jax-dev
}

case "${1:-run}" in
    clean) clean ;;
    run|*) run ;;
esac
