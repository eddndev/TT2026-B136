import { caseId } from './helpers.mjs';
import { summary, notChecked } from '../fixtures/deadline-v2-unit.mjs';

export const deadlinesPath = `/api/v1/cases/${caseId}/deadlines`;
export const profilesPath = `/api/v1/cases/${caseId}/deadline-profiles`;
const allowed = (query, keys) => [...query.keys()].every((key) => keys.includes(key));

export async function installResourceDeadlineReads(page, state, io) {
  const { authorize, reply, fail, unexpected, wait } = io;
  await page.route('**/api/v1/cases/*/deadline-profiles**', async (route) => {
    const call = await authorize(route);
    if (!call) return;
    const parts = call.path.slice(profilesPath.length).split('/').filter(Boolean);
    const query = new URLSearchParams(call.search),
      collection = { kind: 'case', case_id: caseId };
    if (call.method !== 'GET' || call.body !== null || !call.path.startsWith(profilesPath))
      return unexpected(route, call);
    let result;
    if (!parts.length && allowed(query, ['limit', 'status', 'after_id'])) {
      result = {
        collection,
        profiles: state.profiles.map((row) => ({
          id: row.id,
          revision: row.revision,
          status: row.status,
          algorithm: row.algorithm,
          definition_digest: row.definition_digest,
          title: row.definition.title,
          scope: row.scope,
        })),
        has_more: false,
        next_after_id: null,
      };
    } else {
      const rows = state.profiles.filter((row) => row.id === parts[0]);
      if (!rows.length) return fail(route, 'deadline_profile_not_found', 404);
      if (
        parts.length === 2 &&
        parts[1] === 'history' &&
        allowed(query, ['limit', 'before_revision'])
      ) {
        result = {
          collection,
          revisions: rows.map(({ definition, collection: ignored, ...header }) => header),
          has_more: false,
          next_before_revision: null,
        };
      } else if (parts.length === 3 && parts[1] === 'revisions' && !call.search) {
        const row = rows.find((entry) => entry.revision === Number(parts[2]));
        if (!row) return fail(route, 'deadline_profile_not_found', 404);
        result = { ...row, collection };
      } else return unexpected(route, call);
    }
    const snapshot = structuredClone(result);
    await wait(call);
    return reply(route, snapshot);
  });
  await page.route('**/api/v1/cases/*/deadlines**', async (route) => {
    const call = await authorize(route);
    if (!call) return;
    const parts = call.path.slice(deadlinesPath.length).split('/').filter(Boolean);
    const query = new URLSearchParams(call.search);
    if (call.method !== 'GET' || call.body !== null || !call.path.startsWith(deadlinesPath))
      return unexpected(route, call);
    let result;
    if (
      parts.length === 1 &&
      parts[0] === 'responsibles' &&
      allowed(query, ['limit', 'after_id'])
    ) {
      result = {
        case_id: caseId,
        responsibles: state.responsibles,
        has_more: false,
        next_after_id: null,
      };
    } else if (!parts.length && allowed(query, ['limit', 'status', 'after_id'])) {
      result = {
        case_id: caseId,
        deadlines: [...state.deadlineRecords.values()].map((rows) => summary(rows.at(-1))),
        has_more: false,
        next_after_id: null,
      };
    } else if (parts.length === 3 && parts[1] === 'revisions' && !call.search) {
      const row = state.deadlineRecords
        .get(parts[0])
        ?.find((entry) => entry.revision === Number(parts[2]));
      if (!row) {
        await wait(call);
        return fail(route, 'deadline_not_found', 404);
      }
      result = { ...row, operational: notChecked() };
    } else if (parts.length === 1 && !call.search) {
      result = state.deadlineRecords.get(parts[0])?.at(-1);
      if (!result) return fail(route, 'deadline_not_found', 404);
    } else return unexpected(route, call);
    const snapshot = structuredClone(result);
    await wait(call);
    return reply(route, snapshot);
  });
}
