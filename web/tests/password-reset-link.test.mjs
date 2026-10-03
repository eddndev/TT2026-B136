import test from 'node:test';
import assert from 'node:assert/strict';
import { takePasswordResetLink } from '../src/lib/password-reset-link.mjs';

const token = Buffer.alloc(32, 0xfb).toString('base64url');

function browser(hash, fail = false) {
  let location = new URL(`https://qadra.example.test/?view=access${hash}`);
  const calls = [];
  const state = { navigation: 'access' };
  return {
    calls,
    get location() {
      return location;
    },
    get localStorage() {
      throw new Error('recovery must not access storage');
    },
    get sessionStorage() {
      throw new Error('recovery must not access storage');
    },
    history: {
      state,
      replaceState(next, title, url) {
        calls.push({ next, title, url });
        if (fail) throw new Error(`private failed fragment ${token}`);
        location = new URL(url, location);
      },
    },
  };
}

test('a canonical recovery capability is returned only after removing its fragment', () => {
  const page = browser(`#password-reset=${token}`);
  const result = takePasswordResetLink(page);
  assert.equal(page.location.hash, '');
  assert.deepEqual(result, { status: 'ready', token });
  assert.deepEqual(page.calls, [{ next: page.history.state, title: '', url: '/?view=access' }]);
  assert.ok(!JSON.stringify(page.calls).includes(token));
});

test('malformed and noncanonical recovery fragments are removed without admitting a token', () => {
  const invalid = [
    '',
    `${token}=`,
    `${token}&next=cases`,
    `${token.slice(0, -1)}t`,
    Buffer.alloc(31).toString('base64url'),
    Buffer.alloc(33).toString('base64url'),
    `%41${token.slice(1)}`,
    token.replaceAll('-', '+'),
    'a'.repeat(10000),
  ];
  for (const value of invalid) {
    const page = browser(`#password-reset=${value}`);
    assert.deepEqual(takePasswordResetLink(page), { status: 'invalid' });
    assert.equal(page.location.hash, '');
    assert.equal(page.calls.length, 1);
  }
  const bare = browser('#password-reset');
  assert.deepEqual(takePasswordResetLink(bare), { status: 'invalid' });
  assert.equal(bare.location.hash, '');
});

test('ordinary navigation fragments are left untouched', () => {
  for (const hash of ['', '#cases', '#password-reset-history']) {
    const page = browser(hash);
    assert.equal(takePasswordResetLink(page), null);
    assert.equal(page.location.hash, hash);
    assert.equal(page.calls.length, 0);
  }
});

test('failure to remove the fragment does not expose a usable capability or error detail', () => {
  const page = browser(`#password-reset=${token}`, true);
  const result = takePasswordResetLink(page);
  assert.deepEqual(result, { status: 'unavailable' });
  assert.ok(!JSON.stringify(result).includes(token));
});

test('the browser boundary does not cache a token after the fragment is taken', () => {
  const page = browser(`#password-reset=${token}`);
  assert.equal(takePasswordResetLink(page).status, 'ready');
  assert.equal(takePasswordResetLink(page), null);
  assert.equal(page.calls.length, 1);
});
