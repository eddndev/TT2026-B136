import test from 'node:test';
import assert from 'node:assert/strict';
import { readSidebarCollapsed, storeSidebarCollapsed } from '../src/lib/sidebar-state.mjs';

function memory() {
  const values = new Map();
  const storage = {
    getItem: (key) => (values.has(key) ? values.get(key) : null),
    setItem: (key, value) => values.set(key, String(value)),
  };
  return () => storage;
}

test('the sidebar starts expanded and remembers the last choice', () => {
  const storage = memory();
  assert.equal(readSidebarCollapsed(storage), false);
  storeSidebarCollapsed(true, storage);
  assert.equal(readSidebarCollapsed(storage), true);
  storeSidebarCollapsed(false, storage);
  assert.equal(readSidebarCollapsed(storage), false);
});

test('failing storage methods keep the sidebar expanded without throwing', () => {
  const failing = () => ({
    getItem() {
      throw new Error('blocked');
    },
    setItem() {
      throw new Error('blocked');
    },
  });
  assert.equal(readSidebarCollapsed(failing), false);
  assert.doesNotThrow(() => storeSidebarCollapsed(true, failing));
});

test('a throwing or missing storage accessor keeps the sidebar expanded', () => {
  const blocked = () => {
    throw new Error('SecurityError');
  };
  assert.equal(readSidebarCollapsed(blocked), false);
  assert.doesNotThrow(() => storeSidebarCollapsed(true, blocked));
  assert.equal(
    readSidebarCollapsed(() => undefined),
    false,
  );
  assert.doesNotThrow(() => storeSidebarCollapsed(true, () => undefined));
});
