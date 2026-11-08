import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

// Action buttons share one look through src/components/Button.svelte. A raw
// <button> carrying one of the old action classes would bypass it.
const root = new URL('../src/components/', import.meta.url).pathname;
// Whole class names only: a token such as text-ink-secondary is not one.
const legacy =
  /<button\b[^>]*\bclass="(?:[^"]*\s)?(primary|secondary|text-button|icon-button)(?=[\s"])/;

test('action buttons go through the shared Button component', () => {
  const offenders = readdirSync(root, { recursive: true })
    .filter((name) => name.endsWith('.svelte') && !name.endsWith('Button.svelte'))
    .filter((name) => legacy.test(readFileSync(join(root, name), 'utf8')));
  assert.deepEqual(offenders, []);
});
