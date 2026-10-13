import { ownerBase64, ownerCertificateInvalid as invalid } from './owner-certificate-binary.mjs';
import { signingContext, signingUuid } from './individual-signing-values.mjs';
import {
  signingConsent,
  signingSubmission,
  signingVerification,
} from './individual-signing-responses.mjs';
import { signingHash } from './individual-signing-files.mjs';

export function individualSigningApi(request, selected) {
  const context = signingContext(
    selected.accountId,
    selected.caseId,
    selected.documentId,
    selected.version,
    selected.digest,
  );
  const call = (suffix, options) => request('/individual-signing' + suffix, options);
  return {
    async prepare(operationId, pdfSignatureBase64) {
      signingUuid(operationId);
      const digest = await signingHash(ownerBase64(pdfSignatureBase64, 384));
      const value = signingConsent(
        await call('/prepare', {
          method: 'POST',
          data: { operation_id: operationId, pdf_signature_base64: pdfSignatureBase64 },
        }),
        context,
        operationId,
      );
      if (value.pdf_signature_digest !== digest) invalid();
      return value;
    },
    async submit(input) {
      const data = await signingSubmission(input, context);
      const value = signingConsent(
        await call('/submit', { method: 'POST', data }),
        context,
        undefined,
        true,
      );
      if (value.consent_base64 !== data.consent_base64) invalid();
      return value;
    },
    async receipt(operationId, prepared) {
      signingUuid(operationId);
      const value = signingConsent(
        await call('/receipts/' + operationId),
        context,
        operationId,
        true,
      );
      if (prepared && value.consent_base64 !== prepared.consent_base64) invalid();
      return value;
    },
    async verification() {
      return signingVerification(await call('/verification'), context);
    },
  };
}
