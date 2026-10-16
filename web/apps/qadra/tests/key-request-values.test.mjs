import test from 'node:test';
import assert from 'node:assert/strict';
import { keyRequestValues } from '../src/lib/key-request-values.mjs';

const password = 'Local example passphrase';
const draft = (patch = {}) => ({
  commonName: 'Example Holder',
  organization: 'Example Organization',
  country: 'MX',
  password,
  passwordConfirmation: password,
  ...patch,
});
const withPassword = (value) => draft({ password: value, passwordConfirmation: value });

test('request preserves subject and password while omitting confirmation from worker input', () => {
  const input = draft({ commonName: ' Holder ', organization: 'Org\u00e1nica' });
  const before = { ...input };
  assert.deepEqual(keyRequestValues(input), {
    commonName: ' Holder ',
    organization: 'Org\u00e1nica',
    country: 'MX',
    password,
  });
  assert.deepEqual(input, before);
});

test('subject name and organization accept one through 128 UTF-8 bytes independently', () => {
  for (const field of ['commonName', 'organization']) {
    for (const value of ['A', 'a'.repeat(128), '\u00e9'.repeat(64)]) {
      assert.equal(keyRequestValues(draft({ [field]: value }))[field], value);
    }
    for (const value of ['a'.repeat(129), '\u00e9'.repeat(65)]) {
      assert.throws(() => keyRequestValues(draft({ [field]: value })), /128/);
    }
  }
});

test('subject fields reject empty, blank, control-containing and non-text values', () => {
  for (const field of ['commonName', 'organization']) {
    for (const value of ['', '  ', '\t', 'A\nB', 'A\u0000B', 'A\u007fB', 'A\u0085B', null, 4, []]) {
      assert.throws(() => keyRequestValues(draft({ [field]: value })), /nombre|organizaci/i);
    }
  }
});

test('country requires exactly two uppercase ASCII letters without silent normalization', () => {
  for (const country of ['MX', 'US', 'ZZ']) {
    assert.equal(keyRequestValues(draft({ country })).country, country);
  }
  for (const country of ['', 'M', 'MEX', 'mx', 'Mx', 'M1', ' MX', 'MX ', '\u00d1X', null, 12]) {
    assert.throws(() => keyRequestValues(draft({ country })), /pa[i\u00ed]s/i);
  }
});

test('new key password minimum counts Unicode characters rather than UTF-16 units', () => {
  for (const value of ['a'.repeat(12), '\u{1f512}'.repeat(12)]) {
    assert.equal(keyRequestValues(withPassword(value)).password, value);
  }
  for (const value of ['', 'a'.repeat(11), '\u{1f512}'.repeat(11)]) {
    assert.throws(() => keyRequestValues(withPassword(value)), /12/);
  }
});

test('new key password limit is 1024 UTF-8 bytes before crypto work begins', () => {
  for (const value of ['a'.repeat(1024), '\u{1f512}'.repeat(256)]) {
    assert.equal(keyRequestValues(withPassword(value)).password, value);
  }
  for (const value of ['a'.repeat(1025), `${'\u{1f512}'.repeat(256)}x`]) {
    assert.throws(() => keyRequestValues(withPassword(value)), /1024/);
  }
});

test('password and confirmation are exact and preserve surrounding spaces', () => {
  const value = '  local phrase  ';
  assert.equal(keyRequestValues(withPassword(value)).password, value);
  for (const passwordConfirmation of [value.trim(), `${value}x`, '', null, 1, []]) {
    assert.throws(
      () => keyRequestValues(draft({ password: value, passwordConfirmation })),
      /confirmaci/i,
    );
  }
});

test('invalid input does not coerce secrets or echo them in validation errors', () => {
  for (const value of [null, undefined, false, [], {}, 'request']) {
    assert.throws(() => keyRequestValues(value));
  }
  for (const value of [null, undefined, 12, [], {}]) {
    assert.throws(() => keyRequestValues(withPassword(value)), /contrase/i);
  }
  const secret = 'unique example wrong confirmation';
  assert.throws(
    () => keyRequestValues(draft({ password: secret, passwordConfirmation: 'different' })),
    (error) => error instanceof Error && !error.message.includes(secret),
  );
});
