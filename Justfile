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

# build release WASM and assemble site into _site/
build-site:
    wasm-pack build --target web --release
    rm -rf _site
    mkdir -p _site
    cp index.html _site/
    cp -r pkg _site/
    cp -r assets _site/
