build:
    cargo build

build-wasm:
    wasm-pack build --target web

run: build
    cargo run --bin astronomore

serve: build-wasm
    python3 -m http.server 8080

# sanitize the codebase
san:
    cargo fmt
    cargo clippy -- -D warnings

# Install git hooks from .githooks/ (run once after cloning)
install-hooks:
    git config core.hooksPath .githooks
    chmod +x .githooks/pre-commit
    @echo "Hooks installed. Use SKIP_BENCH=1 git commit to skip benchmarks."

# Run benchmarks manually and pretty-print JSON result
bench:
    cargo build --release --bin bench --quiet
    @./target/release/bench | python3 -m json.tool

# Show benchmark history table
bench-history:
    @cat perf/benchmarks.md

# build release WASM and assemble site into _site/
build-site:
    wasm-pack build --target web --release
    rm -rf _site
    mkdir -p _site
    cp index.html _site/
    cp -r pkg _site/
    cp -r assets _site/
