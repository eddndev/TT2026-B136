// Cancel the failed revision only; see docs/adr/0053-ci-failure-cancellation.md.
module.exports = async function cancelCampaign({ github, context, core }) {
  const repo = context.repo;
  const errors = [];
  const allowed = new Set(['.github/workflows/ci.yml', '.github/workflows/web.yml', '.github/workflows/documents.yml']);
  async function cancel(runId) {
    try {
      await github.rest.actions.cancelWorkflowRun({ ...repo, run_id: runId });
      core.info(`Requested cancellation of run ${runId}`);
    } catch (error) {
      if (error.status === 409) core.info(`Run ${runId} already finished or is cancelling`);
      else errors.push(error);
    }
  }
  try {
    const { data: current } = await github.rest.actions.getWorkflowRun({
      ...repo, run_id: context.runId,
    });
    if (allowed.has(current.path) && ['push', 'pull_request'].includes(current.event)) {
      const runs = await github.paginate(github.rest.actions.listWorkflowRunsForRepo, {
        ...repo, head_sha: current.head_sha, per_page: 100,
      });
      for (const run of runs) {
        if (run.id !== context.runId && allowed.has(run.path) &&
            run.path !== current.path && run.head_sha === current.head_sha &&
            run.head_branch === current.head_branch && run.event === current.event &&
            run.status !== 'completed') await cancel(run.id);
      }
    }
  } catch (error) {
    errors.push(error);
  } finally {
    await cancel(context.runId);
  }
  if (errors.length) throw errors[0];
};
