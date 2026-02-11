import { resolve } from 'path';
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';
import react from '@vitejs/plugin-react';

export default defineConfig({
  base: './',
  plugins: [
    react(),
    wasm(),
    topLevelAwait(),
  ],
  server: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  resolve: {
    alias: {
      '@zap/web/react': '/packages/zap-web/src/react/index.ts',
      '@zap/web': '/packages/zap-web/src/index.ts',
    },
  },
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        'react-demo': resolve(__dirname, 'examples/react-demo/index.html'),
        'zap-engine-template': resolve(__dirname, 'examples/zap-engine-template/index.html'),
        'physics-playground': resolve(__dirname, 'examples/physics-playground/index.html'),
        'chemistry-lab': resolve(__dirname, 'examples/chemistry-lab/index.html'),
        'zapzap-mini': resolve(__dirname, 'examples/zapzap-mini/index.html'),
        'glypher': resolve(__dirname, 'examples/glypher/index.html'),
        'flag-parade': resolve(__dirname, 'examples/flag-parade/index.html'),
        'solar-system': resolve(__dirname, 'examples/solar-system/index.html'),
      },
    },
  },
});
