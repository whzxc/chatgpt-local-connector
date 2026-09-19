import { build } from 'vite';
import { rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
// Node is a build tool only. The installed application contains Rust + static UI.
await rm(new URL('../desktop/runtime/', import.meta.url), { recursive: true, force: true });
await build({ configFile: fileURLToPath(new URL('../vite.config.ts', import.meta.url)) });
