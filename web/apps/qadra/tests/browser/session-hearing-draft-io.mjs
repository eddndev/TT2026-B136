import { caseId } from './helpers.mjs';

export function createHearingDraftIo(state) {
  const reply = (route, json, status = 200) =>
    route.fulfill({
      status,
      json,
      headers: { 'Cache-Control': 'no-store' },
    });
  const fail = (route, code, status = 409) => reply(route, { error: { code } }, status);
  const unexpected = (route, call) => {
    state.writes.push(call);
    return fail(route, 'unexpected_hearing_draft_request', 501);
  };
  async function wait(call) {
    const gate = state.hearingGates.find(
      (row) => !row.entered && row.method === call.method && row.path === call.path,
    );
    if (!gate) return;
    gate.entered = true;
    gate.call = call;
    await gate.promise;
  }
  async function authorize(route) {
    const request = route.request(),
      url = new URL(request.url());
    const call = {
      path: url.pathname,
      search: url.search,
      method: request.method(),
      body: request.postData(),
      headers: request.headers(),
      at: state.now,
    };
    state.calls.push(call);
    let code, status;
    if (
      !state.current ||
      call.headers.authorization !== `Bearer ${state.current.token}` ||
      state.now >= Math.min(state.current.absolute, state.current.deadline)
    ) {
      code = 'invalid_session';
      status = 401;
    } else if (!state.allowed || !call.path.startsWith(`/api/v1/cases/${caseId}/`)) {
      code = 'case_not_found';
      status = 404;
    } else if (
      !(
        call.method === 'GET' ? ['owner', 'litigator', 'paralegal'] : ['owner', 'litigator']
      ).includes(state.current.user.role)
    ) {
      code = 'permission_denied';
      status = 403;
    }
    if (code) {
      await fail(route, code, status);
      return null;
    }
    if (state.failures.has(call.path)) {
      await fail(route, 'service_busy', state.failures.get(call.path));
      return null;
    }
    return call;
  }
  return { authorize, reply, fail, unexpected, wait };
}
