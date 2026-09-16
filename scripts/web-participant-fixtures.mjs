// Provision independent accounts for real browser participant permission tests.
import { readFile, writeFile } from 'node:fs/promises';
import { seedTypedParticipants } from './web-typed-participant-fixtures.mjs';

const fixturePath = process.env.TT_WEB_FIXTURES;
const base = process.env.API_PROXY_TARGET;
if (!fixturePath || !base) throw new Error('disposable browser services are required');
const fixture = JSON.parse(await readFile(fixturePath, 'utf8'));

async function request(method, path, body, token, expected = 200) {
  const headers = {};
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  if (token) headers.Authorization = `Bearer ${token}`;
  const response = await fetch(`${base}${path}`, {
    method, headers, body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (response.status !== expected) {
    throw new Error(`participant fixture ${method} ${path}: expected ${expected}, got ${response.status}`);
  }
  return response.status === 204 ? undefined : response.json();
}

const challenge = await request('POST', '/api/v1/auth/login', {
  email: fixture.email, password: fixture.password,
});
const session = await request('POST', '/api/v1/auth/mfa/recovery', {
  challenge_token: challenge.challenge_token, code: fixture.recoveryCodes[7],
});
try {
  const participants = {};
  for (const role of ['owner', 'litigator', 'paralegal', 'client']) {
    const password = `participant browser ${role} password`;
    const enrollment = await request('POST', '/api/v1/users', {
      email: `participants.${role}@example.com`, password, role,
    }, session.access_token, 201);
    participants[role] = {
      id: enrollment.user.id, email: enrollment.user.email, password,
      recoveryCodes: enrollment.recovery_codes,
    };
  }
  for (const [key, title, reference] of [
    ['case', 'Directorio de participantes', 'PARTICIPANTS-001'],
    ['hiddenCase', 'Expediente de acceso restringido', 'PARTICIPANTS-002'],
  ]) {
    const record = await request('POST', '/api/v1/cases', { title, reference }, session.access_token, 201);
    participants[key] = { id: record.id, title: record.title, reference: record.reference };
  }
  for (const role of ['litigator', 'paralegal', 'client']) {
    await request('PUT', `/api/v1/cases/${participants.case.id}/members/${participants[role].id}`,
      undefined, session.access_token, 204);
  }
  fixture.participants = participants;
  fixture.typedParticipants = await seedTypedParticipants(
    request, session.access_token, process.env.TT_LIVE_PARTICIPANT_CERTIFICATE,
  );
  await writeFile(fixturePath, `${JSON.stringify(fixture)}\n`, { mode: 0o600 });
} finally {
  await request('POST', '/api/v1/auth/logout', undefined, session.access_token, 204);
}
