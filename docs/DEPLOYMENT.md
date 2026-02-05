# DEPLOYMENT.md

This guide covers the build pipeline, infrastructure (AWS CDK), and browser compatibility strategies required to deploy **ZapEngine** games to production.

## 1. The Build Pipeline

Unlike standard React apps, ZapEngine requires three distinct artifacts to be built in a specific order.

### The Artifacts

1. **WASM Binary (`.wasm`):** The Rust game logic.
2. **Assets (`.png`, `.mp3`):** Extracted from source files (or `.xcassets`).
3. **Application Bundle (`.js`, `.html`):** The React host and Worker glue.

### The "Golden" Build Sequence

**Critical:** You must extract assets *before* running the Vite build, or `dist/assets` will be empty.

```bash
# 1. Compile Rust to WebAssembly
make wasm
# OR: wasm-pack build crates/zapzap --target web --out-dir src/generated

# 2. Extract Assets (Crucial step often missed)
# Ensures public/assets/ contains the actual images, not just references
python3 scripts/extract_assets.py

# 3. Build React Application
# Copies WASM and public/assets into dist/
npm run build

```

### Verification Before Upload

Before deploying, verify the `dist` folder locally. If these are missing, the game will crash silently or fail to load textures.

```bash
# Check if the heavy assets actually made it
ls -lh dist/assets/
# Check if the WASM is present
ls -lh dist/*.wasm

```

---

## 2. Infrastructure (AWS CDK)

We use AWS CDK to deploy a serverless stack consisting of S3 (Storage) and CloudFront (CDN).

### The "Magic" Headers (SharedArrayBuffer)

To enable high-performance multithreading, the browser enforces strict security isolation. The CloudFront distribution **must** serve these headers on every response, or the `SharedArrayBuffer` will be disabled (forcing the engine into `postMessage` fallback mode).

**Required Headers:**

```http
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp

```

### Deployment Commands

```bash
cd infra

# 1. Install AWS dependencies
npm install

# 2. Bootstrap (First time only per region)
npx cdk bootstrap

# 3. Deploy (Builds dist/ and syncs to S3)
npx cdk deploy

```

### Infrastructure Verification script

Use the `verify-s3.js` script (in root) to ensure assets uploaded correctly and have the correct MIME types.

```bash
node verify-s3.js

```

---

## 3. Browser Compatibility & Hardening

WebGPU is bleeding-edge tech. Deployment requires specific handling for each browser engine to prevent crashes.

### The "Canvas Locking" Issue

**Problem:** If you call `canvas.getContext('webgpu')` and it fails, that canvas element is **permanently locked**. You cannot subsequently call `getContext('2d')` on it.
**Solution:** The engine uses a **Probe Strategy**.

1. Create a disposable `document.createElement('canvas')` off-screen.
2. Attempt WebGPU initialization on the disposable canvas.
3. If successful, mount the real canvas with WebGPU.
4. If failed, mount a *fresh* canvas element with `getContext('2d')`.

### Safari (WebKit) Specifics

* **Pipeline Layouts:** Safari is stricter than Chrome. You cannot have "gaps" in `bindGroupLayouts`. Use an empty layout object instead of `null`.
* **HDR:** Requires `toneMapping: { mode: 'extended' }`.
* **User Action:** Users on Safari 17 or older must enable "WebGPU" in **Develop > Feature Flags**. Safari 18+ works out of the box.

### Firefox (Gecko) Specifics

* **Status:** WebGPU is often behind a flag or blocked by blocklists on Linux/Windows.
* **Fallback:** The engine must catch the `requestAdapter()` promise rejection silently and fallback to Canvas 2D.
* **User Action:** Users must set `dom.webgpu.enabled = true` in `about:config`.

---

## 4. Troubleshooting Production

### Issue: "Failed to get Canvas 2D context"

* **Cause:** The engine tried WebGPU on the main canvas, failed, and then tried 2D on the *same* canvas.
* **Fix:** Ensure `initRenderer` throws an error on failure, and React listens for that error to increment a `rendererKey`, forcing the DOM node to be destroyed and recreated.

### Issue: Game Loads but Screen is Black (Console: `TypeError`)

* **Cause:** Asset fetch failure. The `fetch()` returned a 404 (HTML page) or 403 (XML), but the code tried to pass that text to `createImageBitmap`.
* **Fix:** Check `dist/assets`. Did you run the extraction script?

### Issue: Low FPS / Stuttering

* **Cause:** `SharedArrayBuffer` is disabled because headers are missing.
* **Check:** Open DevTools > Network. Look at the response headers for `index.html`. If `Cross-Origin-Embedder-Policy` is missing, the worker is using slower `postMessage` copying.

### Issue: Colors look washed out (No HDR)

* **Cause:** Browser doesn't support `display-p3` or the monitor is SDR.
* **Fallback:** The engine automatically falls back to `srgb`. This is expected behavior on most non-Apple displays and Firefox.
