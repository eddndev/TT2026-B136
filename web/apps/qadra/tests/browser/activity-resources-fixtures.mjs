import { activityCheckedAt } from '../fixtures/resource-activity-unit.mjs';

export function activityResourcePage(caseId, kind, id, url, views = []) {
  const status = url.searchParams.get('status') || 'linked';
  const after = url.searchParams.get('after_id');
  const limit = Number(url.searchParams.get('limit') || 20);
  const rows = views
    .filter(
      ({ association: row }) =>
        row.selection.target.kind === kind &&
        row.selection.target.id === id &&
        (status === 'all' || row.status === status) &&
        (!after || row.id > after),
    )
    .sort((a, b) => a.association.id.localeCompare(b.association.id));
  const selected = rows.slice(0, limit),
    more = rows.length > limit;
  return {
    case_id: caseId,
    target: { kind, id },
    checked_at: structuredClone(activityCheckedAt),
    associations: selected,
    has_more: more,
    next_after_id: more ? selected.at(-1).association.id : null,
  };
}
