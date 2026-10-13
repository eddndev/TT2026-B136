import { caseId } from './helpers.mjs';
import { summary, historyRow, notChecked } from '../fixtures/deadline-v2-unit.mjs';
export const deadlinesPath = `/api/v1/cases/${caseId}/deadlines`;
export async function installActivityDeadlineReads(
  page,
  state,
  { authorize, reply, fail, unexpected, wait },
) {
  await page.route('**/api/v1/cases/*/deadlines**', async (route) => {
    const call = await authorize(route);
    if (!call) return;
    const parts = call.path.slice(deadlinesPath.length).split('/').filter(Boolean),
      query = new URLSearchParams(call.search);
    if (call.method !== 'GET' || call.body !== null) return unexpected(route, call);
    let result;
    if (!parts.length) {
      if ([...query.keys()].some((key) => !['limit', 'status', 'after_id'].includes(key)))
        return unexpected(route, call);
      result = {
        case_id: caseId,
        deadlines: [summary(state.deadline.at(-1))],
        has_more: false,
        next_after_id: null,
      };
    } else if (parts[0] !== state.deadline[0].id) return fail(route, 'deadline_not_found', 404);
    else if (parts.length === 1 && !call.search) result = state.deadline.at(-1);
    else if (parts.length === 2 && parts[1] === 'history') {
      if ([...query.keys()].some((key) => !['limit', 'before_revision'].includes(key)))
        return unexpected(route, call);
      result = {
        case_id: caseId,
        id: parts[0],
        revisions: [...state.deadline].reverse().map(historyRow),
        has_more: false,
        next_before_revision: null,
      };
    } else if (parts.length === 3 && parts[1] === 'revisions' && !call.search) {
      const row = state.deadline.find((entry) => entry.revision === Number(parts[2]));
      if (row) result = { ...row, operational: notChecked() };
    } else return unexpected(route, call);
    if (!result) return fail(route, 'deadline_not_found', 404);
    const snapshot = structuredClone(result);
    await wait(call);
    return reply(route, snapshot);
  });
}
