import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';

export function requireXcode(env) {
  if (!env.DEVELOPER_DIR && existsSync('/Applications/Xcode.app/Contents/Developer')) {
    env.DEVELOPER_DIR = '/Applications/Xcode.app/Contents/Developer';
  }
  let version;
  try {
    version = execFileSync('xcrun', ['actool', '--version', '--output-format=human-readable-text'],
      { env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  } catch {
    throw new Error('macOS 原生深浅图标需要完整 Xcode 26+；请用 DEVELOPER_DIR 指向 Xcode.app/Contents/Developer，Command Line Tools 不包含 actool。');
  }
  if (Number(version.match(/short-bundle-version:\s*(\d+)/)?.[1] ?? 0) < 26) {
    throw new Error('macOS 原生深浅图标需要 Xcode 26+，当前 actool 版本不满足要求。');
  }
}
