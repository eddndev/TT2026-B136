import test from 'node:test';
import assert from 'node:assert/strict';
import { fitTabs } from '../src/lib/section-tabs-fit.mjs';

test('every tab stays in the row when they all fit', () => {
  assert.deepEqual(fitTabs([100, 100, 100], 364, 32, 80, 0), {
    visible: [0, 1, 2],
    overflow: [],
  });
});

test('trailing tabs move to the overflow menu and leave room for its button', () => {
  // 600 wide: 80 for the button and one gap leave 488, enough for three tabs.
  assert.deepEqual(fitTabs([100, 100, 100, 100, 100, 100], 600, 32, 80, 0), {
    visible: [0, 1, 2],
    overflow: [3, 4, 5],
  });
});

test('the current tab replaces the last ones that fit when it would be hidden', () => {
  assert.deepEqual(fitTabs([100, 100, 100, 100, 100, 180], 600, 32, 80, 5), {
    visible: [0, 1, 5],
    overflow: [2, 3, 4],
  });
});

test('a row too narrow for anything keeps only the current tab', () => {
  assert.deepEqual(fitTabs([100, 100, 100], 120, 32, 80, 1), {
    visible: [1],
    overflow: [0, 2],
  });
});

test('no current tab is forced in when none is selected', () => {
  assert.deepEqual(fitTabs([100, 100, 100], 250, 32, 80, -1), {
    visible: [0],
    overflow: [1, 2],
  });
});
