import { createHash } from 'node:crypto';
function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0).map(([key, item]) => [key, canonical(item)]));
  return value;
}
export function toolFingerprint(tools) {
  const sorted = [...tools].sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
  if (new Set(sorted.map(tool => tool.name)).size !== sorted.length) throw new Error('Duplicate MCP tool names');
  return { toolCount: sorted.length, toolSchemaDigest: createHash('sha256').update(JSON.stringify(canonical(sorted))).digest('hex') };
}
