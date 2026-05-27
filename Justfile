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

# Benchmark the last `COUNT` commits and write perf/range-results.ndjson
bench-range COUNT="10":
    uv run scripts/bench-range.py --count={{ COUNT }}

# Generate perf/bench-report.html (interactive Altair charts) + perf/bench-report.md + perf/plots/*.svg as side-effects
bench-report:
    uvx marimo export html scripts/bench-report.py --output perf/bench-report.html
    @echo "Written: perf/bench-report.html  perf/bench-report.md  perf/plots/"

# Open the benchmark notebook for live interactive exploration. Run with `MODE=edit` to edit.
bench-explore MODE="run":
    uvx marimo {{ MODE }} scripts/bench-report.py

# build release WASM and assemble site into _site/
build-site:
    wasm-pack build --target web --release
    rm -rf _site
    mkdir -p _site
    cp index.html _site/
    cp -r pkg _site/
    cp -r assets _site/
