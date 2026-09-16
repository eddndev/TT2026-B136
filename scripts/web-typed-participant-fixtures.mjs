// Provision typed participant accounts within an existing seeder session.
export async function seedTypedParticipants(request, token, certificatePath) {
  if (!process.env.IDENTITY_TEST_DATABASE_URL || !certificatePath) {
    throw new Error('disposable typed participant services and public certificate are required');
  }
  const participants = { certificatePath };
  for (const role of ['owner', 'litigator', 'paralegal', 'client']) {
    const password = `typed participant browser ${role} password`;
    const enrollment = await request('POST', '/api/v1/users', {
      email: `typed-participants.${role}@example.com`, password, role,
    }, token, 201);
    participants[role] = {
      id: enrollment.user.id, email: enrollment.user.email, password,
      recoveryCodes: enrollment.recovery_codes,
    };
  }
  for (const [key, title, reference] of [
    ['case', 'Participantes tipificados', 'TYPED-PARTICIPANTS-001'],
    ['hiddenCase', 'Identidades de acceso restringido', 'TYPED-PARTICIPANTS-002'],
  ]) {
    const row = await request('POST', '/api/v1/cases', { title, reference }, token, 201);
    participants[key] = { id: row.id, title: row.title, reference: row.reference };
  }
  for (const role of ['litigator', 'paralegal', 'client']) {
    await request('PUT', `/api/v1/cases/${participants.case.id}/members/${participants[role].id}`,
      undefined, token, 204);
  }
  return participants;
}
