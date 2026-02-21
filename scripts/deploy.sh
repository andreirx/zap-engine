#!/usr/bin/env bash
# Deploy to production (S3 + CloudFront)
# Usage: bash scripts/deploy.sh
set -euo pipefail

S3_BUCKET="zapexamplesstack-zapexamplesbucketc673a298-dznsdxx9br2k"
CF_DISTRIBUTION="E2V1S8E7G1IDCZ"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Check dist/ exists
if [ ! -d "dist" ]; then
  echo "Error: dist/ not found. Run 'bash scripts/build-all.sh' first."
  exit 1
fi

echo "==> Syncing dist/ to S3..."
aws s3 sync dist/ "s3://${S3_BUCKET}" --delete

echo "==> Invalidating CloudFront cache..."
INVALIDATION_ID=$(aws cloudfront create-invalidation \
  --distribution-id "${CF_DISTRIBUTION}" \
  --paths "/*" \
  --query 'Invalidation.Id' \
  --output text)

echo "==> Invalidation started: ${INVALIDATION_ID}"
echo "==> Deploy complete!"
echo ""
echo "    Site: https://zapengine.bijup.com"
echo "    Cache invalidation takes 1-5 minutes to propagate."
