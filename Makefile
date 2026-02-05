.PHONY: wasm install dev build test check clean

# Build WASM for the basic demo
wasm:
	wasm-pack build examples/basic-demo --target web --out-dir pkg

# Install npm dependencies
install:
	npm install

# Dev server (requires wasm + install first)
dev: wasm install
	npm run dev

# Production build
build: wasm install
	npm run build

# Run Rust tests
test:
	cargo test --workspace

# Type-check everything
check:
	cargo check --workspace
	cargo check --workspace --target wasm32-unknown-unknown
	npx tsc --noEmit

# Clean build artifacts
clean:
	cargo clean
	rm -rf dist node_modules examples/basic-demo/pkg
