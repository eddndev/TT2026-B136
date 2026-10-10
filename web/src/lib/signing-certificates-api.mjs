import {
  ownerBase64,
  ownerPublicPem,
  ownerCertificateInvalid as invalid,
} from './owner-certificate-binary.mjs';
import { ownerObject } from './owner-certificate-values.mjs';
import { signingUuid, decodeSigningStatement } from './individual-signing-values.mjs';
import { signingBinding } from './individual-signing-responses.mjs';
import { signingHash } from './individual-signing-files.mjs';

export function signingCertificatesApi(request, accountId) {
  signingUuid(accountId);
  let active = true;
  const assertActive = () => {
    if (!active) throw new Error('La consulta del vinculo ya no esta abierta.');
  };
  async function call(suffix, options) {
    assertActive();
    const value = await request('/identity/signing-certificates' + suffix, options);
    assertActive();
    return value;
  }
  async function checked(value, bindingId, receipt) {
    const result = await signingBinding(value, accountId, bindingId, receipt);
    assertActive();
    return result;
  }
  return {
    dispose: () => {
      active = false;
    },
    async current() {
      const value = ownerObject(await call('/current'), ['binding']);
      return value.binding === null ? null : checked(value.binding, undefined, true);
    },
    async get(bindingId) {
      signingUuid(bindingId);
      return checked(await call('/' + bindingId), bindingId, true);
    },
    async prepare(bindingId, certificateBase64) {
      signingUuid(bindingId);
      ownerPublicPem(ownerBase64(certificateBase64, 1, 16384));
      return checked(
        await call('/' + bindingId + '/prepare', {
          method: 'POST',
          data: { certificate_base64: certificateBase64 },
        }),
        bindingId,
        false,
      );
    },
    async register(bindingId, input) {
      signingUuid(bindingId);
      ownerObject(input, ['statement_base64', 'certificate_der_base64', 'signature_base64']);
      const data = Object.freeze({ ...input });
      const canonical = decodeSigningStatement(data.statement_base64, 'binding');
      ownerBase64(data.signature_base64, 384);
      if (
        canonical.bindingId !== bindingId ||
        canonical.accountId !== accountId ||
        (await signingHash(ownerBase64(data.certificate_der_base64, 1, 16384))) !==
          canonical.leafFingerprint
      )
        invalid();
      const result = await checked(
        await call('/' + bindingId + '/register', { method: 'POST', data }),
        bindingId,
        true,
      );
      if (
        result.statement_base64 !== data.statement_base64 ||
        result.signature_base64 !== data.signature_base64 ||
        result.certificate.der_base64 !== data.certificate_der_base64
      )
        invalid();
      return result;
    },
    async withdraw(bindingId) {
      signingUuid(bindingId);
      const result = await checked(
        await call('/' + bindingId + '/withdraw', { method: 'POST' }),
        bindingId,
        true,
      );
      if (result.withdrawn_at === null) invalid();
      return result;
    },
  };
}
