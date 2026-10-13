import {
  resultAnchor,
  resultPrepared,
  resultRecord,
  resultRow,
  resultHistoryRow,
} from '../fixtures/hearing-results.mjs';

export async function installResultDraftRoutes(page, state, io) {
  const { authorize, reply, fail, unexpected, wait } = io;
  state.resultPrepare = (command) => {
    const base = state.results.get(command.result_id)?.at(-1),
      change = command.change;
    const source = state.hearings
      .get(command.hearing_id)
      ?.find((row) => row.revision === change.anchor_revision);
    let continuation = base?.continuation || null;
    if (change.action === 'record' && change.continuation) {
      const previous = state.results
        .get(change.continuation.result_id)
        ?.find((row) => row.revision === change.continuation.revision);
      continuation = {
        hearing_id: previous.hearing_id,
        result_id: previous.id,
        revision: previous.revision,
        values_digest: previous.values_digest,
        submission_digest: previous.receipt.submission_digest,
        status: previous.status,
      };
    }
    const prepared = resultPrepared(
      command,
      base?.anchor || resultAnchor(source),
      base,
      continuation,
    );
    prepared.actor_id = state.current.user.id;
    prepared.attendees = prepared.values.attendees.map((item) => ({
      ...structuredClone(
        state.records.get(item.participant_id).find((row) => row.revision === item.revision),
      ),
      profile: 'manual',
      capacity: item.capacity,
      observation: item.observation,
    }));
    const support = prepared.values.provenance.support;
    prepared.support = support
      ? { ...support, name: 'resultado.pdf', format: 'pdf', policy: 'pdf_docx_v1' }
      : null;
    return prepared;
  };
  state.resultCommit = (prepared) => {
    const row = resultRecord(prepared);
    state.results.set(row.id, [...(state.results.get(row.id) || []), row]);
    return row;
  };
  await page.route('**/api/v1/cases/*/hearings/*/results**', async (route) => {
    const call = await authorize(route);
    if (!call) return;
    const [, hearing, tail] = /\/hearings\/([^/]+)\/results(.*)/.exec(call.path);
    const parts = tail.split('/').filter(Boolean),
      rows = state.results.get(parts[0]);
    const url = new URL(route.request().url());
    if (call.method === 'GET' && call.body === null) {
      if (state.deniedResults.has(parts[0])) return fail(route, 'hearing_result_not_found', 404);
      let result;
      if (!parts.length) {
        const status = url.searchParams.get('status') || 'all';
        result = {
          results: [...state.results.values()]
            .map((rows) => rows.at(-1))
            .filter(
              (row) => row.hearing_id === hearing && (status === 'all' || row.status === status),
            )
            .map(resultRow),
          has_more: false,
          next_after_id: null,
        };
      } else if (!rows || rows[0].hearing_id !== hearing)
        return fail(route, 'hearing_result_not_found', 404);
      else if (parts.length === 1) result = rows.at(-1);
      else if (parts[1] === 'history')
        result = {
          revisions: [...rows].reverse().map(resultHistoryRow),
          has_more: false,
          next_before_revision: null,
        };
      else if (parts.length === 3 && parts[1] === 'revisions')
        result = rows.find((row) => row.revision === Number(parts[2]));
      else return unexpected(route, call);
      if (!result) return fail(route, 'hearing_result_not_found', 404);
      const snapshot = structuredClone(result);
      await wait(call);
      return reply(route, snapshot);
    }
    const body = route.request().postDataJSON(),
      preparing = parts[0] === 'prepare';
    const command = preparing ? body : body.command,
      change = command?.change;
    if (
      !change ||
      command.hearing_id !== hearing ||
      call.search ||
      (preparing ? !state.prepareBudget : !state.nextResultWrite)
    )
      return unexpected(route, call);
    const valid = preparing
      ? call.method === 'POST' && parts.length === 1
      : change.action === 'record'
        ? call.method === 'POST' && parts.length === 0
        : parts[0] === command.result_id &&
          (change.action === 'correct'
            ? call.method === 'PUT' && parts.length === 1
            : change.action === 'withdraw' && call.method === 'POST' && parts[1] === 'withdrawal');
    if (!valid) return unexpected(route, call);
    let mode;
    if (preparing) {
      state.prepareBudget--;
      state.preparations.push({ ...call, values: command });
    } else {
      mode = state.nextResultWrite;
      state.nextResultWrite = null;
      state.resultPosts.push({ ...call, values: body });
    }
    const current = state.results.get(command.result_id)?.at(-1);
    if (state.caseStatus === 'closed') return fail(route, 'case_closed');
    if (change.expected_revision !== (current?.revision || 0))
      return fail(route, 'hearing_result_revision_conflict');
    if (current?.status === 'withdrawn') return fail(route, 'hearing_result_already_withdrawn');
    const prepared = state.resultPrepare(command);
    if (preparing) return reply(route, prepared);
    if (body.expected_submission_digest !== prepared.submission_digest)
      return fail(route, 'hearing_result_submission_mismatch');
    const result = mode.commit === false ? null : state.resultCommit(prepared);
    await wait(call);
    return mode.status ? fail(route, 'service_busy', mode.status) : reply(route, result, 201);
  });
}
