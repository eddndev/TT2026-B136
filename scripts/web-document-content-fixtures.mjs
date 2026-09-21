import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { withCorruptDocumentVault } from './web-document-content-corruption.mjs';

const inbox = '/document-integrity-incidents';

async function findIncident(call, first) {
  let cursor;
  for (let page = 0; page < 20; page++) {
    const result = await call('GET', `${inbox}?limit=100${cursor ? `&after_id=${cursor}` : ''}`);
    const match = result.incidents.find((row) => row.case_id === first.case_id &&
      row.document_id === first.id && row.document_version === first.version);
    if (match) return match;
    if (!result.has_more) break;
    assert.ok(result.next_after_id && result.next_after_id !== cursor);
    cursor = result.next_after_id;
  }
  throw new Error('The real content rejection did not persist its Owner incident');
}

export async function provisionDocumentContent(call) {
  assert.ok(process.env.IDENTITY_TEST_DATABASE_URL && process.env.TT_BACKEND_TEST_DIR);
  const principal = await call('GET', '/auth/me');
  assert.equal(principal.role, 'owner');
  const fixture = {};
  for (const name of ['desktopOwner', 'mobileOwner', 'owner', 'paralegal', 'client']) {
    const role = name.endsWith('Owner') ? 'owner' : name;
    const password = `web document content ${name} synthetic password`;
    const result = await call('POST', '/users', {
      email: `web-document-content-${name.toLowerCase()}@example.com`, password, role,
    }, 201);
    fixture[name] = { id: result.user.id, email: result.user.email, password,
      recoveryCodes: result.recovery_codes };
  }
  for (const key of ['desktop', 'mobile', 'policy']) {
    const record = await call('POST', '/cases', {
      title: `Contenido exacto ${key}`, reference: `CONTENT-${key.toUpperCase()}`,
    }, 201);
    const base = `/cases/${record.id}`;
    const administration = (await call('GET', `${base}/administration`)).administration;
    const scenario = { case: { id: record.id, title: administration.title,
      reference: administration.reference }, closed: key === 'mobile' };
    if (key === 'policy') {
      for (const name of ['paralegal', 'client'])
        await call('PUT', `${base}/members/${fixture[name].id}`, undefined, 204);
    }
    const original = Buffer.concat([Buffer.from(`Historical ${key} content\n`), Buffer.from([0, 255, 13, 10])]);
    const replacement = Buffer.concat([Buffer.from(`Current ${key} content\n`), Buffer.from([1, 254, 10])]);
    const first = await call('POST', `${base}/documents`, original, 201,
      { 'X-Document-Name': `content-${key}.bin` });
    const path = `${base}/documents/${first.id}`;
    await call('POST', `${path}/versions?expected_version=1`, replacement, 201,
      { 'X-Document-Name': `content-${key}-current.bin` });
    scenario.first = await call('GET', `${path}/versions/1`);
    scenario.current = await call('GET', `${path}/versions/2`);
    assert.equal(scenario.first.sealed, false);
    assert.equal(scenario.current.sealed, false);
    assert.equal(scenario.first.digest, createHash('sha256').update(original).digest('hex'));
    assert.equal(scenario.current.digest, createHash('sha256').update(replacement).digest('hex'));
    scenario.payload_base64 = original.toString('base64');
    if (key !== 'policy') {
      await withCorruptDocumentVault(first, async () => {
        const rejected = await call('GET', `${path}/versions/1/content`, undefined, 409);
        assert.equal(rejected.error.code, 'document_content_validation_failed');
      });
      scenario.incident = await findIncident(call, first);
      assert.equal(scenario.incident.failure, 'authentication_failed');
      assert.equal(scenario.incident.requester_id, principal.id);
      assert.equal(scenario.incident.expected_digest, first.digest);
      assert.deepEqual(await call('GET', `${inbox}/${scenario.incident.id}`), scenario.incident);
    }
    if (scenario.closed) await call('PUT', `${base}/administrative-status`, {
      expected_revision: administration.revision, administrative_status: 'closed',
    });
    fixture[key] = scenario;
  }
  return fixture;
}
