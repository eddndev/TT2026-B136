import { profileKinds } from './typed-participant-fields.mjs';
import { identityDraft, supportDraft, decisionDrafts } from './subject-draft-values.mjs';

export const subjectReference = (record) =>
  record
    ? {
        id: record.id,
        revision: record.revision,
        values_digest: record.values_digest,
      }
    : null;

function fieldDraft(value, field) {
  if (['contact', 'protection'].includes(field.type))
    return value.state === 'documented'
      ? { state: value.state, support: supportDraft(value.support) }
      : { state: value.state, reason: value.reason };
  if (field.declared)
    return value.state === 'known'
      ? { state: value.state, value: fieldDraft(value.value, { ...field, declared: false }) }
      : { state: value.state, reason: value.reason };
  return field.type === 'license' ? { number: value.number, issuer: value.issuer } : value;
}

export function typedRoleDraft(value) {
  const fields = profileKinds.find((row) => row.key === value.profile.kind)?.fields ?? [];
  return {
    organization: value.organization,
    legal_status: value.legal_status,
    profile: {
      kind: value.profile.kind,
      ...Object.fromEntries(
        fields.map((field) => [field.key, fieldDraft(value.profile[field.key], field)]),
      ),
    },
    role_support: supportDraft(value.role_support),
  };
}

function proposal(value) {
  return {
    subject:
      value.subject.operation === 'keep'
        ? { operation: 'keep', reference: subjectReference(value.subject.reference) }
        : {
            operation: 'append',
            id: value.subject.id,
            expected_revision: value.subject.expected_revision,
            values: identityDraft(value.subject.values),
          },
    participant_id: value.participant_id,
    expected_participant_revision: value.expected_participant_revision,
    values: {
      subject: subjectReference(value.values.subject),
      directory_status: value.values.directory_status,
      role: typedRoleDraft(value.values.role),
    },
  };
}

export function submissionDraft(bundle) {
  if (!bundle) return null;
  const prepared = bundle.prepared,
    request = bundle.request.prepared;
  return {
    prepared: {
      proposal: { participant_id: prepared.proposal.participant_id },
      submission_revision: prepared.submission_revision,
      submission_digest: prepared.submission_digest,
      declaration: prepared.declaration
        ? {
            digest: prepared.declaration.digest,
            bytes_base64: prepared.declaration.bytes_base64,
            certificate: { fingerprint: prepared.declaration.certificate.fingerprint },
          }
        : null,
    },
    request: {
      signature_base64: bundle.request.signature_base64,
      prepared: {
        proposal: proposal(request.proposal),
        certificate_base64: request.certificate_base64,
        review: {
          directory_stamp: request.review.directory_stamp,
          selection_reason: request.review.selection_reason,
          different: request.review.different.map((row) => ({
            candidate: {
              kind: row.candidate.kind,
              id: row.candidate.id,
              revision: row.candidate.revision,
            },
            reason: row.reason,
            support: supportDraft(row.support),
          })),
        },
      },
    },
  };
}

export function typedCapture(state, form, credential) {
  return {
    subject: identityDraft(state.subject),
    selectedReference: state.selected
      ? subjectReference(state.selected)
      : state.blocked
        ? state.selectedReference
        : null,
    role: typedRoleDraft(state.role),
    reason: state.reason,
    decisions: decisionDrafts(state.decisions),
    expected: state.original?.revision ?? null,
    uncertain: state.uncertain,
    lastSubmission: submissionDraft(state.lastSubmission),
    formDraft: form?.captureDraft() ?? state.formDraft,
    credentialDraft: credential?.captureDraft() ?? state.credentialDraft,
  };
}

export function validateTypedDraft(value, expected) {
  if (
    value?.expected !== expected ||
    typeof value.reason !== 'string' ||
    typeof value.uncertain !== 'boolean' ||
    !value.decisions ||
    (value.uncertain && !value.lastSubmission) ||
    !['natural_person', 'institutional_body'].includes(value.subject?.kind)
  )
    throw new TypeError('Invalid typed participant draft.');
  identityDraft(value.subject);
  typedRoleDraft(value.role);
  decisionDrafts(value.decisions);
}

export function typedSupportEntries(state) {
  const entries = [
    [state.subject, 'identity_support'],
    [state.role, 'role_support'],
    ...Object.values(state.decisions).map((row) => [row, 'support']),
  ];
  for (const field of profileKinds.find((row) => row.key === state.role.profile.kind)?.fields ??
    []) {
    const value = state.role.profile[field.key];
    if (['contact', 'protection'].includes(field.type) && value.state === 'documented')
      entries.push([value, 'support']);
  }
  return entries;
}
