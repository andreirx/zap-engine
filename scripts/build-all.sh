#!/usr/bin/env bash
# Build all WASM crates + Vite production bundle → dist/
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# ── 1. Build WASM for every game crate ─────────────────────────────
CRATES=(
  "examples/basic-demo"
  "examples/zap-engine-template"
  "examples/physics-playground"
  "examples/chemistry-lab"
  "examples/zapzap-mini"
  "examples/glypher"
)

echo "==> Building WASM crates..."
for crate in "${CRATES[@]}"; do
  name=$(basename "$crate")
  echo "    $name"
  wasm-pack build "$crate" --target web --out-dir pkg 2>&1 | tail -1
done

# ── 2. Vite production build ───────────────────────────────────────
echo "==> Installing npm deps..."
npm install --silent

echo "==> Vite build..."
npx vite build

# ── 3. Copy static assets (WASM pkg + public assets) into dist ────
echo "==> Copying WASM packages and static assets..."
for crate in "${CRATES[@]}"; do
  name=$(basename "$crate")

  # Copy wasm-pack output (pkg/)
  if [ -d "$crate/pkg" ]; then
    mkdir -p "dist/$crate/pkg"
    cp "$crate/pkg/"*.{js,wasm,d.ts} "dist/$crate/pkg/" 2>/dev/null || true
  fi

  # Copy public assets
  if [ -d "$crate/public" ]; then
    mkdir -p "dist/$crate/public"
    cp -r "$crate/public/"* "dist/$crate/public/"
  fi
done

echo "==> Build complete! Output in dist/"
echo "    To deploy: cd infra && npm run deploy"
