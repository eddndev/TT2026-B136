import { receipt, ownerId, bindingId, withdrawn } from './owner-certificates.mjs';

export { ownerId, bindingId, withdrawn };
export const token = Buffer.alloc(32, 0xfb).toString('base64url');
export const mfaToken = Buffer.alloc(32, 0x12).toString('base64url');
export const signature = Buffer.alloc(384, 0x5a).toString('base64');
// Exact vector from crates/domain/tests/owner_certificate_login_support/mod.rs.
export const statement = Buffer.from(
  '4f574e41555448310101' +
    '11111111111111111111111111111111' +
    '2222222222222222222222222222222222222222222222222222222222222222' +
    '01020304' +
    '33333333333333333333333333333333' +
    '0102030405060708' +
    '44444444444444444444444444444444' +
    '5555555555555555555555555555555555555555555555555555555555555555' +
    '000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f' +
    '000000006553f100' +
    '000000006553f22c',
  'hex',
);

export const selection = Object.freeze({
  ownerId,
  bindingId,
  leafFingerprint: '55'.repeat(32),
  subject: 'Synthetic public Owner',
});

export function publicReceipt() {
  return receipt();
}

export function challenge(bytes = statement, challengeToken = token) {
  return {
    challenge_token: challengeToken,
    statement_base64: bytes.toString('base64'),
    expires_in_seconds: 299,
  };
}

export function mfa() {
  return { challenge_token: mfaToken, expires_in_seconds: 300 };
}

export function maximumReceipt() {
  const value = publicReceipt();
  const certificate = value.registration.certificate;
  certificate.der_base64 = Buffer.alloc(16384, 1).toString('base64');
  certificate.summary.subject = '\x01'.repeat(65536);
  certificate.summary.issuer = '\x01'.repeat(65536);
  certificate.summary.serial_hex = 'ab'.repeat(20);
  const trust = value.registration.trust;
  trust.root_der_base64 = Buffer.alloc(16384, 2).toString('base64');
  trust.crl_der_base64 = Buffer.alloc(1048576, 3).toString('base64');
  trust.published_by = '\x01'.repeat(1024);
  return value;
}

// Base64 maxima plus worst JSON text escaping; 4096 bounds the fixed DTO envelope,
// counters, timestamps, statement and signature, not another material payload.
export const receiptByteLimit =
  4 * Math.ceil(1048576 / 3) + 8 * Math.ceil(16384 / 3) + 6 * (2 * 65536 + 1024) + 4096;
