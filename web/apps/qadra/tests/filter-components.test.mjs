import { test } from 'node:test';
import assert from 'node:assert/strict';
import { componentMarkup as rendered } from './deadline-editor-render.mjs';

// Server renders of the shared filter components (filter-bar.css). Their
// contents are slots, covered by the browser specs of each screen.
const componentMarkup = async (name, props) =>
  (await rendered(name, props)).replace(/<!--[\s\S]*?-->/g, '');

test('a filter bar renders the requested element with its class', async () => {
  assert.match(await componentMarkup('FilterBar', {}), /^<div class="filter-bar">/);
  const form = await componentMarkup('FilterBar', { as: 'fieldset', class: 'min-w-0' });
  assert.match(form, /^<fieldset class="filter-bar min-w-0">/);
});

test('a filter field sizes its label', async () => {
  assert.match(await componentMarkup('FilterField', {}), /^<label>/);
  assert.match(
    await componentMarkup('FilterField', { size: 'wide' }),
    /^<label class="filter-wide">/,
  );
  const narrow = await componentMarkup('FilterField', { size: 'narrow', class: 'extra' });
  assert.match(narrow, /^<label class="filter-narrow extra">/);
});

test('more filters counts the hidden filters in use', async () => {
  const idle = await componentMarkup('MoreFilters', { count: 0 });
  assert.match(idle, /<details class="filter-more"><summary>M\u00e1s filtros<\/summary>/);
  const used = await componentMarkup('MoreFilters', { count: 2 });
  assert.match(used, /<summary>M\u00e1s filtros \(2\)<\/summary>/);
  const help = await componentMarkup('MoreFilters', { label: 'Ayuda' });
  assert.match(help, /<summary>Ayuda<\/summary>/);
});
