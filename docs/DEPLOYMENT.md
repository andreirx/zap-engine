# Deployment Process

## Infrastructure Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         PRODUCTION STACK                                 │
│                                                                          │
│  ┌─────────────┐      ┌─────────────────┐      ┌──────────────────────┐ │
│  │   Browser   │ ───► │   CloudFront    │ ───► │         S3           │ │
│  │             │      │   E2V1S8E7G1... │      │  zapexamplesstack-   │ │
│  │             │      │                 │      │  zapexamplesbucket-  │ │
│  │             │      │  COOP/COEP      │      │  ...dznsdxx9br2k     │ │
│  │             │      │  headers        │      │                      │ │
│  └─────────────┘      └─────────────────┘      └──────────────────────┘ │
│                                                                          │
│  URL: https://zapengine.bijup.com                                        │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Two Types of Deployment

### 1. Content Update (Most Common)

When you change game code, assets, or examples — infrastructure stays the same.

```bash
# Step 1: Build everything
bash scripts/build-all.sh

# Step 2: Sync to S3
aws s3 sync dist/ s3://zapexamplesstack-zapexamplesbucketc673a298-dznsdxx9br2k --delete

# Step 3: Invalidate CloudFront cache
aws cloudfront create-invalidation --distribution-id E2V1S8E7G1IDCZ --paths "/*"
```

**Or use the deploy script:**
```bash
bash scripts/deploy.sh
```

### 2. Infrastructure Update (Rare)

When you change CDK stack (S3 settings, CloudFront config, headers, domain).

```bash
cd infra
npm run deploy
```

This runs `cdk deploy` which:
- Updates AWS resources if changed
- Deploys `dist/` folder to S3 (built into CDK stack)
- Invalidates CloudFront automatically

---

## Quick Reference

| Task | Command |
|------|---------|
| Build everything | `bash scripts/build-all.sh` |
| Sync to S3 | `aws s3 sync dist/ s3://zapexamplesstack-zapexamplesbucketc673a298-dznsdxx9br2k --delete` |
| Invalidate cache | `aws cloudfront create-invalidation --distribution-id E2V1S8E7G1IDCZ --paths "/*"` |
| Full infra deploy | `cd infra && npm run deploy` |

---

## What Gets Deployed Where

### Source → Build → Deploy

```
examples/*/src/*.rs          (Rust source)
         │
         ▼
    wasm-pack build
         │
         ▼
examples/*/pkg/              (WASM + JS bindings)
         │
         ▼
    vite build
         │
         ▼
dist/                        (Production bundle)
├── index.html
├── assets/                  (JS bundles, hashed filenames)
│   ├── manifest-Su724chU.js
│   ├── TimingBars-C2WAiApP.js
│   └── ...
└── examples/
    ├── solar-system/
    │   ├── index.html
    │   ├── pkg/             (WASM + JS)
    │   │   ├── solar_system_bg.wasm
    │   │   └── solar_system.js
    │   └── public/          (assets.json, images)
    │       └── assets/
    ├── pool-game/
    │   └── ...
    └── ...
         │
         ▼
    aws s3 sync
         │
         ▼
S3: zapexamplesstack-zapexamplesbucketc673a298-dznsdxx9br2k
         │
         ▼
CloudFront: E2V1S8E7G1IDCZ (adds COOP/COEP headers)
         │
         ▼
https://zapengine.bijup.com
```

---

## AWS Resources

| Resource | ID / Name | Purpose |
|----------|-----------|---------|
| S3 Bucket | `zapexamplesstack-zapexamplesbucketc673a298-dznsdxx9br2k` | Stores all static files |
| CloudFront | `E2V1S8E7G1IDCZ` | CDN, HTTPS, COOP/COEP headers |
| ACM Cert | `arn:aws:acm:us-east-1:324037297014:certificate/ebf61d12-...` | SSL for zapengine.bijup.com |
| Domain | `zapengine.bijup.com` | Custom domain |

---

## CloudFront Headers

CloudFront adds these headers required for SharedArrayBuffer:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

Without these, browsers block SharedArrayBuffer and the engine falls back to slow postMessage.

---

## Cache Invalidation

CloudFront caches files. After S3 sync, you MUST invalidate:

```bash
aws cloudfront create-invalidation --distribution-id E2V1S8E7G1IDCZ --paths "/*"
```

Invalidation takes 1-5 minutes to propagate globally.

To check status:
```bash
aws cloudfront get-invalidation --distribution-id E2V1S8E7G1IDCZ --id <INVALIDATION_ID>
```

---

## Deployment Checklist

```
□ 1. Build: bash scripts/build-all.sh
□ 2. Test locally: npm run dev → check examples work
□ 3. Sync: aws s3 sync dist/ s3://zapexamplesstack-... --delete
□ 4. Invalidate: aws cloudfront create-invalidation ...
□ 5. Verify: https://zapengine.bijup.com
```

---

## Troubleshooting

### Changes not appearing on production

1. Did you run `build-all.sh`? Check `dist/` has fresh files.
2. Did you sync to S3? Check with `aws s3 ls s3://zapexamplesstack-.../examples/solar-system/pkg/`
3. Did you invalidate CloudFront? Check invalidation status.
4. Browser cache? Hard refresh (Cmd+Shift+R).

### SharedArrayBuffer not working on production

CloudFront must send COOP/COEP headers. Verify:
```bash
curl -I https://zapengine.bijup.com | grep -i cross-origin
```

Should show:
```
cross-origin-opener-policy: same-origin
cross-origin-embedder-policy: require-corp
```

If missing, check CDK stack's `ResponseHeadersPolicy`.

### S3 Access Denied

The bucket is private (CloudFront-only). You can't access S3 URLs directly. Always use the CloudFront URL.
