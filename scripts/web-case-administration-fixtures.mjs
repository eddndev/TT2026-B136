// Provision case administration scenarios only in disposable browser services.
import { randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';

const fixturePath = process.env.TT_WEB_FIXTURES;
const base = process.env.API_PROXY_TARGET;
const database = process.env.IDENTITY_TEST_DATABASE_URL;
if (!fixturePath || !base || !database) {
  throw new Error('disposable browser services are required');
}
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
async function request(method, path, body, token, expected = 200) {
  const headers = {};
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  if (token) headers.Authorization = `Bearer ${token}`;
  const response = await fetch(`${base}${path}`, {
    method, headers, body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (response.status !== expected) {
    throw new Error(`case fixture ${method} ${path}: expected ${expected}, got ${response.status}`);
  }
  return response.status === 204 ? undefined : response.json();
}
const challenge = await request('POST', '/api/v1/auth/login', {
  email: fixture.email, password: fixture.password,
});
const session = await request('POST', '/api/v1/auth/mfa/recovery', {
  challenge_token: challenge.challenge_token, code: fixture.recoveryCodes[5],
});
try {
  const administration = {};
  for (const role of ['owner', 'litigator', 'paralegal', 'client']) {
    const password = `case administration browser ${role} password`;
    const enrollment = await request('POST', '/api/v1/users', {
      email: `case-administration.${role}@example.com`, password, role,
    }, session.access_token, 201);
    administration[role] = {
      id: enrollment.user.id, email: enrollment.user.email, password,
      recoveryCodes: enrollment.recovery_codes,
    };
  }
  for (const [key, title, reference] of [
    ['basicCase', 'Expediente con ficha pendiente', 'ADMIN-BASIC-001'],
    ['hiddenCase', 'Expediente penal restringido', 'ADMIN-HIDDEN-001'],
  ]) {
    const record = await request('POST', '/api/v1/cases', { title, reference }, session.access_token, 201);
    administration[key] = { id: record.id, title: record.title, reference: record.reference };
  }
  const legacy = {
    id: randomUUID(), title: 'Expediente anterior sin revisiones', reference: 'ADMIN-LEGACY-001',
  };
  // Administrative fixture: a pre-revision baseline has no invented event or R1.
  execFileSync('psql', [database, '-X', '-q', '-v', 'ON_ERROR_STOP=1',
    '-v', `case_id=${legacy.id}`, '-v', `title=${legacy.title}`,
    '-v', `reference=${legacy.reference}`, '-v', `creator=${administration.owner.id}`], {
    input: `INSERT INTO cases (id,title,reference,created_by,required_initial_revision)
      VALUES (:'case_id',:'title',:'reference',:'creator',NULL);\n`,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  administration.legacyCase = legacy;
  for (const key of ['basicCase', 'legacyCase']) {
    for (const role of ['owner', 'litigator', 'paralegal', 'client']) {
      await request('PUT', `/api/v1/cases/${administration[key].id}/members/${administration[role].id}`,
        undefined, session.access_token, 204);
    }
  }
  fixture.caseAdministration = administration;
  await writeFile(fixturePath, `${JSON.stringify(fixture)}\n`, { mode: 0o600 });
} finally {
  await request('POST', '/api/v1/auth/logout', undefined, session.access_token, 204);
}
