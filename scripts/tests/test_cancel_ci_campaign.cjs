const { test } = require('node:test');
const assert = require('node:assert/strict');
const cancelCampaign = require('../cancel-ci-campaign.cjs');

function fixture(overrides = {}) {
  const current = {
    id: 10, path: '.github/workflows/ci.yml', head_sha: 'abc',
    head_branch: 'feat/change', event: 'pull_request', status: 'in_progress',
    ...overrides,
  };
  const calls = [];
  const list = Symbol('list');
  const github = {
    rest: { actions: {
      getWorkflowRun: async () => ({ data: current }),
      listWorkflowRunsForRepo: list,
      cancelWorkflowRun: async ({ run_id }) => { calls.push(run_id); },
    } },
    paginate: async (method, params) => {
      assert.equal(method, list);
      assert.equal(params.head_sha, current.head_sha);
      return [];
    },
  };
  const args = { github, context: { runId: 10, repo: { owner: 'owner', repo: 'repo' } },
    core: { info() {}, warning() {} } };
  return { current, calls, github, args };
}

test('cancels only active CI/Web peers on the same revision, branch and event, then itself', async () => {
  const f = fixture();
  const peer = { ...f.current, id: 20, path: '.github/workflows/web.yml' };
  f.github.paginate = async () => [
    f.current, peer, { ...peer, id: 21, status: 'queued' },
    { ...peer, id: 30, head_sha: 'newer' },
    { ...peer, id: 31, head_branch: 'main' },
    { ...peer, id: 32, event: 'push' },
    { ...peer, id: 33, status: 'completed' },
    { ...peer, id: 34, path: '.github/workflows/deploy.yml' },
    { ...f.current, id: 35 },
  ];
  await cancelCampaign(f.args);
  assert.deepEqual(f.calls, [20, 21, 10]);
});

test('a manual measurement cancels itself without looking up other campaigns', async () => {
  const f = fixture({ event: 'workflow_dispatch' });
  f.github.paginate = async () => assert.fail('manual campaign must not enumerate peers');
  await cancelCampaign(f.args);
  assert.deepEqual(f.calls, [10]);
});

test('completed-run races do not prevent cancelling the current run', async () => {
  const f = fixture();
  f.github.paginate = async () => [{ ...f.current, id: 20, path: '.github/workflows/web.yml' }];
  f.github.rest.actions.cancelWorkflowRun = async ({ run_id }) => {
    f.calls.push(run_id);
    if (run_id === 20) throw Object.assign(new Error('already completed'), { status: 409 });
  };
  await cancelCampaign(f.args);
  assert.deepEqual(f.calls, [20, 10]);
});

test('a failed peer lookup still attempts current-run cancellation and reports failure', async () => {
  const f = fixture();
  f.github.paginate = async () => { throw new Error('lookup failed'); };
  await assert.rejects(cancelCampaign(f.args), /lookup failed/);
  assert.deepEqual(f.calls, [10]);
});

test('one peer permission error does not leave other targets running silently', async () => {
  const f = fixture();
  f.github.paginate = async () => [20, 21].map(id => ({
    ...f.current, id, path: '.github/workflows/web.yml',
  }));
  f.github.rest.actions.cancelWorkflowRun = async ({ run_id }) => {
    f.calls.push(run_id);
    if (run_id === 20) throw Object.assign(new Error('permission denied'), { status: 403 });
  };
  await assert.rejects(cancelCampaign(f.args), /permission denied/);
  assert.deepEqual(f.calls, [20, 21, 10]);
});
