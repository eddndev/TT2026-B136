import test from 'node:test';
import assert from 'node:assert/strict';
import { Blob, File } from 'node:buffer';
import { descriptor, registry, attach, suspended, recover } from './fixtures/draft-registry.mjs';

test('suspension captures synchronously and disposal preserves pending values', async () => {
  const store = registry();
  let mounted = true,
    captures = 0;
  const handle = store.register(descriptor(), {
    fields: ['text'],
    capture: () => {
      assert.equal(mounted, true);
      captures++;
      return { text: 'raw' };
    },
  });
  const report = store.suspend();
  assert.deepEqual(report.captured, [handle.key]);
  assert.equal(captures, 1);
  assert.equal(store.active(), false);
  assert.deepEqual(store.pending(), []);
  mounted = false;
  handle.dispose();
  store.activate('account-a');
  assert.equal(store.pending().length, 1);
  assert.deepEqual(await recover(store, handle.key), { text: 'raw' });
  assert.deepEqual(store.pending(), []);
});

test('projection excludes unselected credentials, callbacks and private server rows', async () => {
  const store = registry();
  const source = { text: 'keep this', privateRows: [{ email: 'private@example.test' }] };
  Object.defineProperty(source, 'access_token', {
    get() {
      throw new Error('must not read');
    },
  });
  source.callback = () => assert.fail('must not invoke');
  const handle = store.register(descriptor(), { fields: ['text'], capture: () => source });
  suspended(store);
  assert.deepEqual(await recover(store, handle.key), { text: 'keep this' });
});

test('copy preserves partial text, undefined, NaN and exact File and Blob material', async () => {
  const store = registry();
  const values = {
    draft: { text: '  invalid +', optional: undefined, offset: NaN, rows: [{ pending: '-' }] },
    file: new File(['attachment'], 'evidence.txt', { type: 'text/plain', lastModified: 1234 }),
    certificate: new Blob(['public certificate'], { type: 'application/octet-stream' }),
  };
  const handle = attach(store, descriptor(), values);
  suspended(store);
  values.draft.text = 'changed';
  values.draft.rows[0].pending = 'changed';
  const saved = await recover(store, handle.key);
  assert.equal(saved.draft.text, '  invalid +');
  assert.equal(Object.hasOwn(saved.draft, 'optional'), true);
  assert.equal(saved.draft.optional, undefined);
  assert.equal(Number.isNaN(saved.draft.offset), true);
  assert.equal(saved.draft.rows[0].pending, '-');
  assert.ok(saved.file instanceof File);
  assert.equal(saved.file.name, 'evidence.txt');
  assert.equal(saved.file.type, 'text/plain');
  assert.equal(saved.file.lastModified, 1234);
  assert.equal(await saved.file.text(), 'attachment');
  assert.equal(saved.certificate.type, 'application/octet-stream');
  assert.equal(await saved.certificate.text(), 'public certificate');
});

test('known nested secrets and runtime objects reject the complete projection', () => {
  const invalid = [
    ...[
      'access_token',
      'password',
      'challenge_token',
      'totp_secret',
      'recovery_codes',
      'Authorization',
    ].map((name) => ({ nested: [{ [name]: 'secret' }] })),
    { nested: () => {} },
    { nested: Promise.resolve('pending') },
    { api: { send() {} } },
    { callback() {} },
    { busy: true },
    new (class Client {
      constructor() {
        this.url = '/api';
      }
    })(),
  ];
  for (const value of invalid) {
    const store = registry();
    const handle = attach(store, descriptor(), { text: 'must not silently drop', value });
    const report = suspended(store);
    assert.deepEqual(report.captured, []);
    assert.deepEqual(
      report.failed.map((failure) => failure.key),
      [handle.key],
    );
    assert.deepEqual(store.pending(), []);
  }
});

for (const field of ['totp_secret_base32', 'otpauth_uri']) {
  test(`enrollment field ${field} cannot enter a business draft`, () => {
    const store = registry();
    const handle = attach(store, descriptor(), { draft: { [field]: 'credential material' } });
    const report = suspended(store);
    assert.deepEqual(report.captured, []);
    assert.deepEqual(
      report.failed.map((failure) => failure.key),
      [handle.key],
    );
    assert.deepEqual(store.pending(), []);
  });
}

test('a failed or asynchronous capture reports loss but still permits suspension', async () => {
  for (const capture of [
    () => {
      throw new Error('capture failed');
    },
    () => Promise.resolve({ text: 'late' }),
  ]) {
    const store = registry();
    const bad = store.register(descriptor(), { fields: ['text'], capture });
    const good = attach(store, descriptor({ resourceId: 'document-b' }));
    const report = store.suspend();
    assert.equal(store.active(), false);
    assert.deepEqual(report.captured, [good.key]);
    assert.deepEqual(
      report.failed.map((failure) => failure.key),
      [bad.key],
    );
    store.activate('account-a');
    assert.deepEqual(await recover(store, good.key), { text: 'unfinished' });
  }
});
