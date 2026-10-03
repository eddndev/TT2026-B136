import {
  OWNER_CERTIFICATE_LIMIT,
  ownerBase64,
  ownerPublicPem,
  ownerCertificateInvalid as invalid,
} from './owner-certificate-binary.mjs';
import {
  ownerUuid,
  ownerPreparation,
  ownerReceipt,
  ownerSubmission,
} from './owner-certificate-values.mjs';
import { reconcileOwnerIntent } from './owner-certificate-intent.mjs';

export function ownerCertificatesApi(request, ownerId) {
  ownerUuid(ownerId);
  let active = true;
  const base = '/auth/certificate-bindings';
  const assertActive = () => {
    if (!active) throw new Error('La consulta del certificado ya no esta abierta.');
  };
  async function call(suffix, options) {
    assertActive();
    try {
      const value = await request(base + suffix, options);
      assertActive();
      return value;
    } catch (error) {
      assertActive();
      throw error;
    }
  }
  return {
    dispose() {
      active = false;
    },
    async current() {
      const value = await call('/current');
      return value === null ? null : ownerReceipt(value, ownerId, undefined, true);
    },
    async get(bindingId) {
      ownerUuid(bindingId);
      return ownerReceipt(await call(`/${bindingId}`), ownerId, bindingId);
    },
    async prepare(bindingId, certificateBase64) {
      ownerUuid(bindingId);
      ownerPublicPem(ownerBase64(certificateBase64, 1, OWNER_CERTIFICATE_LIMIT));
      const value = await call(`/${bindingId}/prepare`, {
        method: 'POST',
        data: { certificate_base64: certificateBase64 },
      });
      return ownerPreparation(value, ownerId, bindingId);
    },
    async register(bindingId, input) {
      ownerUuid(bindingId);
      const data = ownerSubmission(input);
      const value = ownerReceipt(
        await call(`/${bindingId}/register`, {
          method: 'POST',
          data,
        }),
        ownerId,
        bindingId,
      );
      if (
        reconcileOwnerIntent({ ownerId, bindingId, operation: 'register', data }, value) !==
        'matched'
      )
        invalid();
      return value;
    },
    async withdraw(bindingId, expectedRevision) {
      ownerUuid(bindingId);
      if (expectedRevision !== 1) invalid();
      const value = ownerReceipt(
        await call(`/${bindingId}/withdraw`, {
          method: 'POST',
          data: { expected_revision: expectedRevision },
        }),
        ownerId,
        bindingId,
      );
      if (value.revision !== 2) invalid();
      return value;
    },
  };
}
