// Provision stage workflows only against disposable browser services.
import { readFile, writeFile } from 'node:fs/promises';

const fixturePath = process.env.TT_WEB_FIXTURES;
const base = process.env.API_PROXY_TARGET;
if (!fixturePath || !base || !process.env.IDENTITY_TEST_DATABASE_URL) {
  throw new Error('disposable browser services are required');
}
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
let token;
async function request(method, path, body, expected = 200) {
  const response = await fetch(`${base}/api/v1${path}`, {
    method,
    headers: {
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(body !== undefined ? { 'Content-Type': 'application/json' } : {}),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (response.status !== expected) {
    throw new Error(`stage fixture ${method} ${path}: expected ${expected}, got ${response.status}`);
  }
  return response.status === 204 ? undefined : response.json();
}
const challenge = await request('POST', '/auth/login', {
  email: fixture.email, password: fixture.password,
});
const session = await request('POST', '/auth/mfa/recovery', {
  challenge_token: challenge.challenge_token, code: fixture.recoveryCodes[6],
});
token = session.access_token;
try {
  const stages = {};
  for (const role of ['owner', 'litigator', 'paralegal', 'client']) {
    const password = `case stages browser ${role} password`;
    const enrollment = await request('POST', '/users', {
      email: `case-stages.${role}@example.com`, password, role,
    }, 201);
    stages[role] = {
      id: enrollment.user.id, email: enrollment.user.email, password,
      recoveryCodes: enrollment.recovery_codes,
    };
  }
  for (const [key, title, reference] of [
    ['legacyCase', 'Expediente incorporado sin etapa', 'STAGE-ADOPTION-001'],
    ['hiddenCase', 'Expediente procesal restringido', 'STAGE-HIDDEN-001'],
  ]) {
    const record = await request('POST', '/cases', { title, reference }, 201);
    const before = await request('GET', `/cases/${record.id}/administration`);
    const updated = await request('PUT', `/cases/${record.id}/administration`, {
      expected_revision: before.administration.revision, title, reference,
      profile: {
        nuc: `${reference}-NUC`, nuc_authority: 'Fiscalia declarada',
        judicial_case_number: `${reference}-CJ`, judicial_authority: 'Organo declarado',
        offenses: ['Descripcion declarada'],
        general_information: null, complementary_identifiers: null,
      },
    });
    if (updated.initial_stage !== null) {
      throw new Error('stage adoption fixture must have no initial registration');
    }
    stages[key] = { id: record.id, title, reference };
  }
  for (const role of ['owner', 'litigator', 'paralegal', 'client']) {
    await request('PUT', `/cases/${stages.legacyCase.id}/members/${stages[role].id}`, undefined, 204);
  }
  fixture.caseStages = stages;
  await writeFile(fixturePath, `${JSON.stringify(fixture)}\n`, { mode: 0o600 });
} finally {
  await request('POST', '/auth/logout', undefined, 204);
}
