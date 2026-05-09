build:
    cargo build

build-wasm:
    wasm-pack build --target web

run: build
    cargo run

serve: build-wasm
    python3 -m http.server 8080

# sanitize the codebase
san:
    cargo fmt
    cargo clippy -- -D warnings