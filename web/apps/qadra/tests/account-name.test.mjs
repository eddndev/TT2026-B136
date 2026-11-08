import test from 'node:test';
import assert from 'node:assert/strict';
import { accountName, accountNameMax, accountInitials } from '../src/lib/account-name.mjs';

test('an account name is trimmed and keeps its interior spaces', () => {
  assert.equal(accountNameMax, 120);
  assert.equal(accountName('  Ana  Garc\u00eda '), 'Ana  Garc\u00eda');
  assert.equal(accountName('\u00e9'.repeat(120)), '\u00e9'.repeat(120));
  // Length counts Unicode scalars, so an astral character counts once.
  assert.equal(accountName('\u{1F600}'.repeat(120)), '\u{1F600}'.repeat(120));
  assert.equal(accountName(' x'.repeat(60) + '  '), 'x'.repeat(60).split('').join(' '));
});

test('an empty, oversized or non-string account name is refused', () => {
  for (const value of ['', '   ', 'a'.repeat(121), '\u{1F600}'.repeat(121)])
    assert.throws(() => accountName(value), /nombre/i);
  for (const value of [undefined, null, 7, {}, ['Ana']])
    assert.throws(() => accountName(value), /nombre/i);
});

test('control, format and line separator characters are refused before trimming', () => {
  for (const value of ['a\u0000b', '\u0009Ana', 'Ana\u000a', '\u0085Ana', '\u2028Ana', 'Ana\u2029'])
    assert.throws(() => accountName(value), /nombre/i);
  // One scalar from each format (Cf) range.
  for (const code of [
    0xad, 0x600, 0x61c, 0x6dd, 0x70f, 0x890, 0x8e2, 0x180e, 0x200b, 0x200e, 0x202a, 0x202e, 0x2060,
    0x2066, 0xfeff, 0xfff9, 0x110bd, 0x110cd, 0x13430, 0x1bca0, 0x1d173, 0xe0001, 0xe0020,
  ])
    assert.throws(() => accountName(`Ana${String.fromCodePoint(code)}`), /nombre/i, `U+${code}`);
});

test('account initials come from the first two words of the name', () => {
  const email = 'hatz@example.com';
  assert.equal(accountInitials({ name: 'Hatziel Ortega', email }), 'HO');
  assert.equal(accountInitials({ name: 'hatziel ortega rivera', email }), 'HO');
  assert.equal(accountInitials({ name: 'Ana', email }), 'A');
  assert.equal(accountInitials({ name: '\u00e9lan  vital', email }), '\u00c9V');
  assert.equal(accountInitials({ name: '\u{1F600} Ana', email }), '\u{1F600}A');
  assert.equal(accountInitials({ email }), 'HA');
  assert.equal(accountInitials({ name: '', email }), 'HA');
});
