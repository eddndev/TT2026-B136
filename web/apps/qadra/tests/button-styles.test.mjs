import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buttonClass } from '../src/lib/button-styles.mjs';

test('each variant carries its marker class and the shared base', () => {
  for (const variant of ['primary', 'secondary', 'text', 'icon']) {
    const value = buttonClass(variant);
    assert.match(value, new RegExp(`\\bbtn-${variant}\\b`));
    assert.match(value, /\binline-flex\b/);
  }
  assert.match(buttonClass(), /\bbtn-secondary\b/);
  assert.match(buttonClass('primary', 'w-full'), /\bw-full$/);
  assert.throws(() => buttonClass('danger'), /Unknown button variant/);
});
