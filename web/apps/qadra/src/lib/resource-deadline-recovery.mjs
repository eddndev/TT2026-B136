import {
  resourceDeadlineDraft,
  resourceDeadlineRecordsMatch,
} from './resource-deadline-values.mjs';
async function exact(read, missingCode) {
  try {
    return await read();
  } catch (error) {
    if (error.status === 404 && error.code === missingCode) return null;
    throw error;
  }
}
export async function readResourceDeadlineSubmission(deadlines, associations, raw) {
  const draft = resourceDeadlineDraft(raw),
    c = draft.command;
  const deadline = await exact(
    () => deadlines.revision(c.deadline.deadline_id, 1),
    'deadline_not_found',
  );
  const view = await exact(
    () => associations.revision(c.association_id, 1),
    'resource_activity_not_found',
  );
  if (!deadline && !view) return { state: 'absent' };
  if (!deadline || !view) return { state: 'different' };
  const records = { deadline, association: view.association };
  return resourceDeadlineRecordsMatch(records, draft)
    ? { state: 'paired' }
    : { state: 'different' };
}
