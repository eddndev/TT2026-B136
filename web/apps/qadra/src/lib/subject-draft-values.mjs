function declared(value) {
  if (value.state === 'known') return { state: value.state, value: value.value };
  return { state: value.state, reason: value.reason };
}

export function supportDraft(value) {
  if (value === null) return null;
  return {
    document_id: value.document_id,
    version: value.version,
    digest: value.digest,
    locator: value.locator,
    ...(value.name === undefined ? {} : { name: value.name }),
  };
}

export function identityDraft(value) {
  const identity_support = supportDraft(value.identity_support);
  if (value.kind === 'institutional_body')
    return {
      kind: value.kind,
      name: value.name,
      institutional_identifier: declared(value.institutional_identifier),
      identity_support,
    };
  const name =
    value.name.state === 'known'
      ? { state: value.name.state, value: value.name.value }
      : { state: value.name.state, label: value.name.label, reason: value.name.reason };
  return { kind: value.kind, name, curp: declared(value.curp), identity_support };
}

export function decisionDrafts(values) {
  return Object.fromEntries(
    Object.entries(values).map(([key, value]) => [
      key,
      {
        reason: value.reason,
        support: supportDraft(value.support),
      },
    ]),
  );
}

export function subjectCapture({ draft, reason, decisions, expected, uncertain, last }) {
  return {
    draft: identityDraft(draft),
    reason,
    decisions: decisionDrafts(decisions),
    expected,
    uncertain,
    last: last
      ? {
          expected: last.expected,
          values: identityDraft(last.values),
          review: {
            directory_stamp: last.review.directory_stamp,
            selection_reason: last.review.selection_reason,
            different: last.review.different.map((entry) => ({
              candidate: {
                kind: entry.candidate.kind,
                id: entry.candidate.id,
                revision: entry.candidate.revision,
              },
              reason: entry.reason,
              support: supportDraft(entry.support),
            })),
          },
        }
      : null,
  };
}

export function validateSubjectDraft(value, expected) {
  if (
    value?.expected !== expected ||
    typeof value.reason !== 'string' ||
    typeof value.uncertain !== 'boolean' ||
    !value.decisions ||
    !['natural_person', 'institutional_body'].includes(value.draft?.kind) ||
    (value.uncertain && value.last?.expected !== expected)
  )
    throw new TypeError('Invalid subject draft.');
  subjectCapture(value);
}
