import { build } from 'vite';
import { rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { buildNative } from './build-native.mjs';
// Node is a build tool only. The installed application contains Rust + static UI.
await rm(new URL('../desktop/runtime/', import.meta.url), { recursive: true, force: true });
await build({ configFile: fileURLToPath(new URL('../vite.config.ts', import.meta.url)) });

const args = process.argv.slice(2);
await buildNative({ release: !args.includes('--debug'), target: args.includes('--target') ? args[args.indexOf('--target') + 1] : undefined });
