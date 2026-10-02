import { basicFields, profileFields } from './case-administration.mjs';

export function caseEditorDescriptor(principalId, caseId, action, instanceId, expected) {
  return {
    principalId,
    contextId: caseId ?? 'case-creation',
    editorKind: 'case-administration',
    resourceId: caseId ?? null,
    action,
    instanceId,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: expected,
  };
}

export function sameCaseEditor(left, right) {
  return (
    ['principalId', 'contextId', 'editorKind', 'resourceId', 'action'].every(
      (field) => left[field] === right[field],
    ) &&
    left.ownerDraftKey === null &&
    left.fieldPath.length === 0 &&
    left.rowId === null
  );
}

export function validateCaseEditorDraft(value, expected) {
  if (
    value.expected !== expected ||
    (expected !== null && (!Number.isSafeInteger(expected) || expected < 0)) ||
    typeof value.complete !== 'boolean' ||
    typeof value.uncertain !== 'boolean' ||
    basicFields.some(({ key }) => typeof value.draft?.[key] !== 'string') ||
    profileFields.some(({ key }) => typeof value.draft?.profile?.[key] !== 'string') ||
    !Array.isArray(value.draft?.profile?.offenses) ||
    value.draft.profile.offenses.some((item) => typeof item !== 'string') ||
    typeof value.rawFields?.offenses?.pending !== 'string' ||
    (value.submitted !== null &&
      (typeof value.submitted?.nuc !== 'string' ||
        typeof value.submitted?.judicial_case_number !== 'string'))
  )
    throw new TypeError('Invalid case editor draft.');
}
