# infra/ — AWS CDK Deployment

Deploys all ZapEngine examples as a single static site to S3 + CloudFront.

## Architecture

```
S3 Bucket (private) ← BucketDeployment (../dist)
    ↓ OAC
CloudFront Distribution
    → COOP/COEP headers (required for SharedArrayBuffer)
    → HTTPS enforced
    → Gzip/Brotli compression
```

## Files

| File | Purpose |
|------|---------|
| `bin/app.ts` | CDK app entry point |
| `lib/zap-examples-stack.ts` | Stack: S3 bucket, CloudFront, headers, deployment |
| `cdk.json` | CDK config (points to `bin/app.ts`) |

## Usage

```bash
# From repo root — build everything and deploy
make deploy

# Or manually:
bash scripts/build-all.sh    # Build WASM + Vite → dist/
cd infra && npx cdk deploy   # Deploy dist/ to AWS
```

## Key Headers

- `Cross-Origin-Opener-Policy: same-origin` — required for SharedArrayBuffer
- `Cross-Origin-Embedder-Policy: require-corp` — required for SharedArrayBuffer
