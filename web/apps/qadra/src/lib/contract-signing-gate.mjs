// Decisions of the contract signing step that do not depend on the page.
// signRefusal says how a refused /sign answer leaves the step: a stale
// statement is discarded and prepared again, a refusal that wrote nothing
// keeps the signature for a resend, and an answer without a definite refusal
// stays uncertain until the receipt of the operation is read.
// Rationale: docs/adr/0086-bilateral-contract-signing.md.

const STALE = ['signing_account_changed', 'signing_trust_changed', 'contract_revision_conflict'];
const NOTHING_WRITTEN = ['contract_timestamp_unavailable', 'signing_trust_unavailable'];

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
