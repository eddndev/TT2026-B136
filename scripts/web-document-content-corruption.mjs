import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { join } from 'node:path';

function sql(statement) {
  try {
    return execFileSync('psql', ['-X', '-qAt', '-v', 'ON_ERROR_STOP=1',
      '-d', process.env.IDENTITY_TEST_DATABASE_URL], {
      input: statement, encoding: 'utf8', timeout: 30000,
      env: { ...process.env, PGDATABASE: process.env.IDENTITY_TEST_DATABASE_URL },
      stdio: ['pipe', 'pipe', 'pipe'],
    }).trim();
  } catch {
    throw new Error('Disposable content fixture SQL failed; no data was logged');
  }
}

function requireDisposableDatabase() {
  const location = new URL(process.env.IDENTITY_TEST_DATABASE_URL);
  assert.equal(location.protocol, 'postgresql:');
  assert.equal(location.hostname, '127.0.0.1');
  assert.equal(location.username, 'tt_backend_test_admin');
  assert.equal(location.pathname, '/postgres');
  assert.match(location.port, /^[1-9][0-9]{0,4}$/);
  assert.ok(process.env.TT_BACKEND_TEST_DIR);
  const [database, directory, user, superuser, port] = sql(
    "SELECT current_database(),current_setting('data_directory'),current_user," +
    "current_setting('is_superuser'),current_setting('port')",
  ).split('|');
  assert.equal(database, 'postgres');
  assert.equal(user, 'tt_backend_test_admin');
  assert.equal(superuser, 'on');
  assert.equal(port, location.port);
  assert.equal(realpathSync(directory), realpathSync(join(process.env.TT_BACKEND_TEST_DIR, 'postgres')));
}

export async function withCorruptDocumentVault(row, operation) {
  requireDisposableDatabase();
  const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
  assert.match(row.id, uuid);
  assert.match(row.case_id, uuid);
  assert.ok(Number.isInteger(row.version) && row.version >= 1 && row.version <= 4294967295);
  const where = `id='${row.id}' AND case_id='${row.case_id}' AND version=${row.version}`;
  const read = () => sql(`SELECT encode(vault,'hex') FROM documents WHERE ${where}`);
  const original = read();
  assert.match(original, /^(?:[0-9a-f]{2})+$/);
  const changed = Buffer.from(original, 'hex');
  assert.ok(changed.length > 97);
  changed[changed.length - 1] ^= 1;
  function replace(value) {
    assert.match(value, /^(?:[0-9a-f]{2})+$/);
    assert.equal(sql('BEGIN; SET LOCAL session_replication_role=replica; ' +
      `UPDATE documents SET vault=decode('${value}','hex') WHERE ${where} RETURNING id; COMMIT;`), row.id);
  }
  try {
    // This fixture-only transaction deliberately bypasses immutable-row guards.
    replace(changed.toString('hex'));
    return await operation();
  } finally {
    // Restore exactly the fixture bytes, not a product repair operation. The
    // HTTP rejection's persistent incident is retained for browser inspection.
    replace(original);
    assert.equal(read(), original);
  }
}
