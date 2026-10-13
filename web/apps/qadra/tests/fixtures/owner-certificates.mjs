import { readFileSync } from 'node:fs';

// Public transport fixtures do not establish a Partner certificate admission.
export const publicPem = readFileSync(
  new URL('./participant-public-certificate.pem', import.meta.url),
);
export const publicDer = Buffer.from(
  publicPem.toString().replace(/-----[^-]+-----|\s/g, ''),
  'base64',
);
export const ownerId = '33333333-3333-3333-3333-333333333333';
export const bindingId = '44444444-4444-4444-4444-444444444444';
export const signature = Buffer.alloc(384, 7);
export const canonical = Buffer.from(
  '4f574e43455254310101' +
    '11111111111111111111111111111111' +
    '2222222222222222222222222222222222222222222222222222222222222222' +
    '01020304' +
    '33333333333333333333333333333333' +
    '1112131415161718' +
    '0102030405060708' +
    '44444444444444444444444444444444' +
    '00000000' +
    '00000001' +
    '5555555555555555555555555555555555555555555555555555555555555555',
  'hex',
);

export function preparation(owner = ownerId, binding = bindingId) {
  const statement = Buffer.from(canonical);
  Buffer.from(owner.replaceAll('-', ''), 'hex').copy(statement, 62);
  Buffer.from(binding.replaceAll('-', ''), 'hex').copy(statement, 94);
  return {
    binding_id: binding,
    owner_id: owner,
    policy: 'internal_partner_binding_v1',
    statement_base64: statement.toString('base64'),
    account_revision: BigInt('0x1112131415161718').toString(),
    auth_generation: BigInt('0x0102030405060708').toString(),
    deployment_id: '11111111-1111-1111-1111-111111111111',
    trust_revision: 0x01020304,
    root_fingerprint: '22'.repeat(32),
    certificate: {
      der_base64: publicDer.toString('base64'),
      fingerprint: '55'.repeat(32),
      summary: {
        subject: 'Synthetic public Owner',
        issuer: 'Synthetic root',
        serial_hex: '1234',
        not_before_unix: 100,
        not_after_unix: 2000000000,
      },
    },
  };
}

export function submission(prepared = preparation(), signed = signature.toString('base64')) {
  return {
    statement_base64: prepared.statement_base64,
    certificate_der_base64: prepared.certificate.der_base64,
    signature_base64: signed,
  };
}

export function receipt(prepared = preparation(), signed = signature.toString('base64')) {
  return {
    binding_id: prepared.binding_id,
    owner_id: prepared.owner_id,
    revision: 1,
    policy: prepared.policy,
    withdrawal: null,
    registration: {
      statement_base64: prepared.statement_base64,
      statement_digest: '66'.repeat(32),
      certificate: structuredClone(prepared.certificate),
      signature_base64: signed,
      account_revision: prepared.account_revision,
      auth_generation: prepared.auth_generation,
      checked_at_unix: 1000,
      valid_from_unix: 100,
      valid_until_unix: 2000000000,
      registered_at: '2026-10-02T18:00:01.123456789Z',
      trust: {
        deployment_id: prepared.deployment_id,
        revision: prepared.trust_revision,
        root_der_base64: Buffer.from('public-root-double').toString('base64'),
        crl_der_base64: Buffer.from('public-crl-double').toString('base64'),
        root_fingerprint: prepared.root_fingerprint,
        crl_digest: '77'.repeat(32),
        crl_number: '9007199254740993',
        crl_this_update_unix: 100,
        crl_next_update_unix: 2000000000,
        valid_from_unix: 100,
        valid_until_unix: 2000000000,
        published_at: '2026-10-02T17:00:00Z',
        published_by: 'synthetic-administrator',
      },
    },
  };
}

export function withdrawn(original = receipt()) {
  const result = structuredClone(original);
  const statement = Buffer.from(result.registration.statement_base64, 'base64');
  statement[8] = 2;
  statement.writeUInt32BE(1, 110);
  statement.writeUInt32BE(2, 114);
  result.revision = 2;
  result.withdrawal = {
    statement_base64: statement.toString('base64'),
    account_revision: result.registration.account_revision,
    auth_generation: result.registration.auth_generation,
    withdrawn_at: '2026-10-02T18:00:02.987654321Z',
  };
  return result;
}
