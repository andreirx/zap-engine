#!/usr/bin/env npx tsx
// Asset Baker — scans an image folder and outputs an assets.json manifest.
//
// Usage:
//   npx tsx tools/bake-assets.ts <input-dir> [--output path/to/assets.json]
//
// Convention:
//   - Files named `*_NxM.ext` (e.g., hero_4x8.png) → atlas with cols=N, rows=M
//   - Files without `_NxM` suffix → single-sprite atlas (1x1)
//   - Single-sprite atlases get one named sprite (filename without extension)
//   - Multi-cell atlases get sprites named "{name}_{col}_{row}" for each cell

import * as fs from 'node:fs';
import * as path from 'node:path';

const IMAGE_EXTENSIONS = new Set(['.png', '.jpg', '.jpeg', '.webp']);

// Matches filenames like "hero_4x8.png" → name="hero", cols=4, rows=8
const ATLAS_PATTERN = /^(.+)_(\d+)x(\d+)$/;

interface AtlasDescriptor {
  name: string;
  cols: number;
  rows: number;
  path: string;
}

interface SpriteDescriptor {
  atlas: number;
  col: number;
  row: number;
}

interface AssetManifest {
  atlases: AtlasDescriptor[];
  sprites: Record<string, SpriteDescriptor>;
}

function parseArgs(argv: string[]): { inputDir: string; outputPath: string } {
  const args = argv.slice(2);
  let inputDir = '';
  let outputPath = '';

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--output' && i + 1 < args.length) {
      outputPath = args[++i];
    } else if (!args[i].startsWith('-')) {
      inputDir = args[i];
    }
  }

  if (!inputDir) {
    console.error('Usage: npx tsx tools/bake-assets.ts <input-dir> [--output assets.json]');
    process.exit(1);
  }

  if (!outputPath) {
    outputPath = path.join(inputDir, 'assets.json');
  }

  return { inputDir, outputPath };
}

function bake(inputDir: string): AssetManifest {
  const resolvedDir = path.resolve(inputDir);

  if (!fs.existsSync(resolvedDir)) {
    console.error(`Error: directory not found: ${resolvedDir}`);
    process.exit(1);
  }

  const files = fs.readdirSync(resolvedDir)
    .filter(f => IMAGE_EXTENSIONS.has(path.extname(f).toLowerCase()))
    .sort();

  if (files.length === 0) {
    console.error(`Error: no image files found in ${resolvedDir}`);
    process.exit(1);
  }

  const atlases: AtlasDescriptor[] = [];
  const sprites: Record<string, SpriteDescriptor> = {};

  for (const file of files) {
    const ext = path.extname(file);
    const stem = path.basename(file, ext);
    const match = stem.match(ATLAS_PATTERN);

    let name: string;
    let cols: number;
    let rows: number;

    if (match) {
      name = match[1];
      cols = parseInt(match[2], 10);
      rows = parseInt(match[3], 10);
    } else {
      name = stem;
      cols = 1;
      rows = 1;
    }

    const atlasIndex = atlases.length;
    atlases.push({ name, cols, rows, path: file });

    if (cols === 1 && rows === 1) {
      // Single sprite — use the atlas name directly
      sprites[name] = { atlas: atlasIndex, col: 0, row: 0 };
    } else {
      // Multi-cell atlas — generate named sprites for each cell
      for (let row = 0; row < rows; row++) {
        for (let col = 0; col < cols; col++) {
          const spriteName = `${name}_${col}_${row}`;
          sprites[spriteName] = { atlas: atlasIndex, col, row };
        }
      }
    }
  }

  return { atlases, sprites };
}

function main() {
  const { inputDir, outputPath } = parseArgs(process.argv);
  const manifest = bake(inputDir);

  const json = JSON.stringify(manifest, null, 2) + '\n';
  fs.writeFileSync(outputPath, json, 'utf-8');

  const spriteCount = Object.keys(manifest.sprites).length;
  console.log(`Baked ${manifest.atlases.length} atlas(es), ${spriteCount} sprite(s) → ${outputPath}`);
}

main();
