import test from 'node:test';
import assert from 'node:assert/strict';
import {
  bytesBase64,
  base64Bytes,
  matchesSubmission,
  typedParticipantFailure,
} from '../src/lib/typed-participant-preparation.mjs';
test('converts binary data without text reserialization', async () => {
  const bytes = Uint8Array.from([0, 255, 128, 10]);
  assert.equal(await bytesBase64(new Blob([bytes])), 'AP+ACg==');
  assert.deepEqual(base64Bytes('AP+ACg=='), bytes);
});
test('reconciles unsigned submissions only by exact participant and origin revision and digest', () => {
  const prepared = {
    proposal: { participant_id: 'id' },
    submission_revision: 4,
    submission_digest: 'a',
  };
  const record = { id: 'id', submission_revision: 4, submission_digest: 'a' };
  assert.equal(matchesSubmission(record, prepared), true);
  for (const change of [{ id: 'other' }, { submission_revision: 5 }, { submission_digest: 'b' }])
    assert.equal(matchesSubmission({ ...record, ...change }, prepared), false);
});
test('reconciles signed declarations only with their exact original public evidence', () => {
  const prepared = {
    proposal: { participant_id: 'id' },
    submission_revision: 4,
    declaration: { digest: 'a', bytes_base64: 'statement', certificate: { fingerprint: 'cert' } },
  };
  const record = {
    id: 'id',
    credential_origin: { participant_id: 'id', participant_revision: 4, statement_digest: 'a' },
  };
  const evidence = {
    reference: record.credential_origin,
    statement_digest: 'a',
    declaration_base64: 'statement',
    certificate_fingerprint: 'cert',
    signature_base64: 'signature',
  };
  assert.equal(matchesSubmission(record, prepared, 'signature', evidence), true);
  assert.equal(matchesSubmission(record, prepared, 'other', evidence), false);
  assert.equal(matchesSubmission(record, prepared, 'signature'), false);
  assert.equal(
    matchesSubmission(record, prepared, 'signature', { ...evidence, declaration_base64: 'other' }),
    false,
  );
});
test('capacity and exhausted revisions do not advise changing identifiers or retrying as CAS', () => {
  assert.match(typedParticipantFailure({ code: 'participant_candidate_limit' }), /capacidad/);
  assert.match(typedParticipantFailure({ code: 'participant_revision_exhausted' }), /agot/);
  assert.match(typedParticipantFailure({ status: 503 }), /confirmar/);
});

test('credential failures identify revoked certificates and invalid signatures without treating them as permissions', () => {
  assert.match(
    typedParticipantFailure({ code: 'participant_credential_revoked', status: 422 }),
    /revocado/,
  );
  assert.match(
    typedParticipantFailure({ code: 'participant_credential_invalid_signature', status: 422 }),
    /firma.*corresponde/,
  );
});

test('public credential rejection codes give actionable Spanish guidance', () => {
  const cases = [
    ['untrusted_issuer', /certificado.*CA interna/],
    ['malformed_certificate', /certificado.*formato/],
    ['unsupported_certificate', /certificado.*perfil/],
    ['malformed_crl', /lista de revocaci\u00f3n.*formato.*administrador/],
    ['unsupported_crl', /lista de revocaci\u00f3n.*perfil.*administrador/],
    ['untrusted_crl', /lista de revocaci\u00f3n.*CA interna.*administrador/],
    ['crl_not_yet_valid', /lista de revocaci\u00f3n.*todav\u00eda.*administrador/],
    ['crl_expired', /lista de revocaci\u00f3n.*vencida.*administrador/],
    ['limit_exceeded', /material.*l\u00edmites.*administrador/],
  ];
  for (const [suffix, guidance] of cases) {
    const code = `participant_credential_${suffix}`;
    assert.match(
      typedParticipantFailure({ code, status: 422, message: 'credential validation failed' }),
      guidance,
      code,
    );
  }
});
