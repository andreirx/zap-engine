.PHONY: wasm wasm-all install dev build build-all test check clean deploy

# Build WASM for the basic demo (quick iteration)
wasm:
	wasm-pack build examples/basic-demo --target web --out-dir pkg

# Build WASM for all game crates
wasm-all:
	wasm-pack build examples/basic-demo --target web --out-dir pkg
	wasm-pack build examples/zap-engine-template --target web --out-dir pkg
	wasm-pack build examples/physics-playground --target web --out-dir pkg
	wasm-pack build examples/chemistry-lab --target web --out-dir pkg
	wasm-pack build examples/zapzap-mini --target web --out-dir pkg
	wasm-pack build examples/glypher --target web --out-dir pkg
	wasm-pack build examples/flag-parade --target web --out-dir pkg
	wasm-pack build examples/solar-system --target web --out-dir pkg

# Install npm dependencies
install:
	npm install

# Dev server (requires wasm + install first)
dev: wasm install
	npm run dev

# Production build (single example — basic demo)
build: wasm install
	npm run build

# Full production build — all examples (WASM + Vite + static assets)
build-all:
	bash scripts/build-all.sh

# Deploy to AWS (requires build-all first)
deploy: build-all
	cd infra && npm install && npx cdk deploy

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
	rm -rf dist node_modules
	rm -rf examples/basic-demo/pkg
	rm -rf examples/zap-engine-template/pkg
	rm -rf examples/physics-playground/pkg
	rm -rf examples/chemistry-lab/pkg
	rm -rf examples/zapzap-mini/pkg
	rm -rf examples/glypher/pkg
	rm -rf examples/flag-parade/pkg
	rm -rf examples/solar-system/pkg
