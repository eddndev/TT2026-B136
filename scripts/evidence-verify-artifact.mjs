// Check that the generated evidence validation module matches its sources.
// The source digest covers the wrapper crate, the verification engine, the
// workspace manifests and the build scripts, so any engine change marks the
// module stale. With --write it records the current digests instead.
import { createHash } from 'node:crypto';
import { readFile, writeFile, readdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve, relative } from 'node:path';
const root = fileURLToPath(new URL('..', import.meta.url));
const output = resolve(root, 'web/apps/qadra/src/generated/evidence-verify');
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
async function tree(path) {
  const found = [];
  for (const entry of await readdir(path, { withFileTypes: true })) {
    const full = resolve(path, entry.name);
    if (entry.isDirectory()) found.push(...await tree(full));
    else if (entry.isFile()) found.push(full);
  }
  return found.sort();
}
const sources = [
  'Cargo.toml', 'Cargo.lock', 'scripts/build-evidence-verify.sh',
  'scripts/browser-crypto-toolchain.py', 'scripts/evidence-verify-artifact.mjs',
].map((path) => resolve(root, path));
sources.push(...await tree(resolve(root, 'crates/evidence-verify-wasm')));
sources.push(...await tree(resolve(root, 'crates/evidence-verify')));
const entries = await Promise.all(sources.sort().map(async (path) =>
  [relative(root, path), digest(await readFile(path))]));
const source = digest(JSON.stringify(entries));
const assets = Object.fromEntries(await Promise.all(
  ['evidence_verify_wasm.js', 'evidence_verify_wasm_bg.wasm'].map(async (name) =>
    [name, digest(await readFile(resolve(output, name)))])));
const expected = { source, assets };
if (process.argv.includes('--write')) {
  await writeFile(resolve(output, 'manifest.json'), JSON.stringify(expected, null, 2) + '\n');
} else {
  const actual = JSON.parse(await readFile(resolve(output, 'manifest.json'), 'utf8'));
  if (JSON.stringify(actual) !== JSON.stringify(expected))
    throw new Error('Evidence validation artifact is stale. Run bash scripts/build-evidence-verify.sh.');
}
