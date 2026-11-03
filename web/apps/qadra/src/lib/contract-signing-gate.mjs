// Decisions of the contract signing step that do not depend on the page.
// partialAccepted admits the local validation report of the partial
// acknowledgement only when it is that profile, did not fail and covers the
// exact CTRTREQ1 bytes of this request, so another package cannot unlock the
// counter-signature. signRefusal says how a refused /sign answer leaves the
// step: a stale statement is discarded and prepared again, a refusal that
// wrote nothing keeps the signature for a resend, and an answer without a
// definite refusal stays uncertain until the receipt of the operation is read.
// Rationale: docs/adr/0086-bilateral-contract-signing.md.

export const PARTIAL_PROFILE = 'bilateral-contract-partial-v1';
export const REQUEST_ENTRY = 'contrato-solicitud.bin';
const ACCEPTED = ['checked', 'indeterminate'];
const STALE = ['signing_account_changed', 'signing_trust_changed', 'contract_revision_conflict'];
const NOTHING_WRITTEN = ['contract_timestamp_unavailable', 'signing_trust_unavailable'];

export function partialAccepted(report, requestDigest) {
  if (!report || typeof requestDigest !== 'string' || requestDigest.length !== 64) return false;
  if (report.profile !== PARTIAL_PROFILE || !ACCEPTED.includes(report.overall)) return false;
  const digests = Array.isArray(report.digests) ? report.digests : [];
  return digests.some((entry) => entry?.name === REQUEST_ENTRY && entry.sha256 === requestDigest);
}

export function signRefusal(failure) {
  const code = failure?.code;
  const status = failure?.status;
  const refused = Number.isInteger(status) && status < 500;
  if (STALE.includes(code))
    return {
      mode: 'editing',
      keepSignature: false,
      stale: true,
      reload: code === 'contract_revision_conflict',
    };
  if (NOTHING_WRITTEN.includes(code))
    return { mode: 'editing', keepSignature: true, stale: false, reload: false };
  if (refused)
    return {
      mode: 'editing',
      keepSignature: code === 'signing_conflict',
      stale: false,
      reload: false,
    };
  return { mode: 'uncertain', keepSignature: true, stale: false, reload: false };
}
