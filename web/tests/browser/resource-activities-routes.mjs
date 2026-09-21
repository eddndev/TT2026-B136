import { failFact } from './procedural-facts-helpers.mjs';
import { caseId } from './helpers.mjs';
import { summary, historyRow } from '../fixtures/deadline-v2-unit.mjs';

export async function installResourceActivityRoutes(page, state, options) {
  const { resources, resource, deadline } = state;
  resources.handle = async (route, call) => {
    if (!call.path.includes('/activities')) return false;
    state.calls.push(call);
    if (state.handle && (await state.handle(route, call))) return true;
    const url = new URL(route.request().url());
    const parts = url.pathname.split('/activities')[1].split('/').filter(Boolean);
    if (call.method !== 'GET') {
      if (options.role === 'paralegal') {
        await failFact(route, 'permission_denied', 403);
        return true;
      }
      if (options.closed) {
        await failFact(route, 'case_closed');
        return true;
      }
      const command = parts[0] === 'prepare' ? call.body : call.body.command;
      const current = state.records.get(command.association_id)?.at(-1);
      if (
        command.expected_resource_revision !== resources.records.get(resource.id).at(-1).revision
      ) {
        await failFact(route, 'resource_activity_resource_revision_conflict');
        return true;
      }
      if ((current?.revision || 0) !== command.change.expected_revision) {
        await failFact(route, 'resource_activity_revision_conflict');
        return true;
      }
      const draft = state.prepare(command);
      if (parts[0] === 'prepare') {
        await route.fulfill({ json: draft });
        return true;
      }
      state.submissions.push(command);
      if (draft.submission_digest !== call.body.expected_submission_digest) {
        await failFact(route, 'resource_activity_submission_mismatch');
        return true;
      }
      await route.fulfill({ status: 201, json: state.commit(draft) });
      return true;
    }
    if (!parts.length) {
      const rows = [...state.records.values()]
        .map((v) => v.at(-1))
        .filter(
          (row) =>
            (!url.searchParams.has('kind') ||
              row.selection.target.kind === url.searchParams.get('kind')) &&
            (!url.searchParams.has('status') || row.status === url.searchParams.get('status')) &&
            (!url.searchParams.has('after_id') || row.id > url.searchParams.get('after_id')),
        )
        .sort((a, b) => a.id.localeCompare(b.id));
      const limit = Number(url.searchParams.get('limit')),
        selected = rows.slice(0, limit),
        more = rows.length > limit;
      await route.fulfill({
        json: {
          case_id: caseId,
          resource_id: resource.id,
          associations: selected.map(state.view),
          has_more: more,
          next_after_id: more ? selected.at(-1).id : null,
        },
      });
      return true;
    }
    const rows = state.records.get(parts[0]);
    if (!rows) {
      await failFact(route, 'resource_activity_not_found', 404);
      return true;
    }
    if (parts[1] === 'history') {
      const rowsBefore = [...rows]
        .reverse()
        .filter(
          (row) =>
            !url.searchParams.has('before_revision') ||
            row.revision < Number(url.searchParams.get('before_revision')),
        );
      const limit = Number(url.searchParams.get('limit')),
        selected = rowsBefore.slice(0, limit),
        more = rowsBefore.length > limit;
      await route.fulfill({
        json: {
          case_id: caseId,
          resource_id: resource.id,
          association_id: parts[0],
          revisions: selected,
          has_more: more,
          next_before_revision: more ? selected.at(-1).revision : null,
        },
      });
      return true;
    }
    const row =
      parts[1] === 'revisions' ? rows.find((v) => v.revision === Number(parts[2])) : rows.at(-1);
    if (!row) await failFact(route, 'resource_activity_not_found', 404);
    else await route.fulfill({ json: state.view(row) });
    return true;
  };
  await page.route('**/api/v1/cases/*/deadlines**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const parts = url.pathname.split('/deadlines')[1].split('/').filter(Boolean);
    state.targetCalls.push({ path: url.pathname, method: request.method() });
    if (resources.facts.results.scheduling.denied || options.role === 'client')
      return failFact(route, 'permission_denied', 403);
    if (request.method() !== 'GET') return failFact(route, 'invalid_input', 400);
    if (!parts.length)
      return route.fulfill({
        json: {
          case_id: caseId,
          deadlines: [summary(deadline.at(-1))],
          has_more: false,
          next_after_id: null,
        },
      });
    if (parts[0] !== deadline[0].id) return failFact(route, 'deadline_not_found', 404);
    if (parts[1] === 'history')
      return route.fulfill({
        json: {
          case_id: caseId,
          id: parts[0],
          revisions: [...deadline].reverse().map(historyRow),
          has_more: false,
          next_before_revision: null,
        },
      });
    const row =
      parts[1] === 'revisions'
        ? deadline.find((v) => v.revision === Number(parts[2]))
        : deadline.at(-1);
    return row ? route.fulfill({ json: row }) : failFact(route, 'deadline_not_found', 404);
  });
}
