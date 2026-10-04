export function createHearingDerivedDeadlineActions({
  read,
  update,
  admitted,
  scoped,
  buildCommand,
  finish,
  fail,
}) {
  let alive = true,
    generation = 0;
  const current = (expected) => alive && generation === expected && admitted();
  const available = (value) =>
    alive && admitted() && !value.busy && !value.pending && !value.disabled && !value.blocked;
  function begin(patch) {
    const expected = ++generation;
    update({ busy: true, error: '', ...patch });
    return expected;
  }
  function conflict() {
    update({
      mode: 'conflict',
      acknowledged: false,
      retryAvailable: false,
      error:
        'La revision recibida no coincide con el envio conservado. Consulta de nuevo su resultado.',
    });
  }
  async function prepare() {
    const value = read();
    if (!available(value) || value.closed || value.mode !== 'draft' || value.last) return;
    const expected = begin({ prepared: null, acknowledged: false, retryAvailable: false });
    try {
      const command = structuredClone(buildCommand()),
        actor = structuredClone(value.user);
      const response = await scoped.prepare(command, actor);
      if (!current(expected)) return;
      if (response.state === 'replay') await finish(response.record, true);
      else if (response.state === 'ready')
        update({ prepared: structuredClone(response), mode: 'review' });
      else throw new Error('No se pudo confirmar la preparacion conjunta.');
    } catch (error) {
      if (current(expected)) fail(error, false);
    } finally {
      if (current(expected)) update({ busy: false });
    }
  }
  async function submit() {
    const value = read();
    if (
      !available(value) ||
      value.closed ||
      value.mode !== 'review' ||
      !value.prepared ||
      !value.acknowledged ||
      value.last
    )
      return;
    const last = structuredClone(value.prepared),
      actor = structuredClone(value.user);
    const expected = begin({ last, mode: 'uncertain', acknowledged: false, retryAvailable: false });
    try {
      const response = await scoped.submit(structuredClone(last), actor);
      if (current(expected)) await finish(response, false);
    } catch (error) {
      if (current(expected)) fail(error, true);
    } finally {
      if (current(expected)) update({ busy: false });
    }
  }
  async function check() {
    const value = read();
    if (!available(value) || !['uncertain', 'conflict'].includes(value.mode) || !value.last) return;
    const last = structuredClone(value.last),
      actor = structuredClone(value.user);
    const expected = begin({ retryAvailable: false, acknowledged: false });
    try {
      const response = await scoped.prepare(structuredClone(last.command), actor);
      if (!current(expected)) return;
      const digest =
        response.state === 'replay' ? response.record.review_digest : response.review_digest;
      if (!['ready', 'replay'].includes(response.state) || digest !== last.review_digest) {
        conflict();
        return;
      }
      if (response.state === 'replay') await finish(response.record, true);
      else
        update({
          mode: 'uncertain',
          retryAvailable: !read().closed,
          error:
            'No se observo un registro conjunto. Esto no confirma que el envio fallo. Puedes consultar de nuevo o repetir explicitamente el mismo envio.',
        });
    } catch (error) {
      if (current(expected)) fail(error, true);
    } finally {
      if (current(expected)) update({ busy: false });
    }
  }
  async function retry() {
    const value = read();
    if (
      !available(value) ||
      value.closed ||
      value.mode !== 'uncertain' ||
      !value.last ||
      !value.retryAvailable
    )
      return;
    const last = structuredClone(value.last),
      actor = structuredClone(value.user);
    const expected = begin({ retryAvailable: false, acknowledged: false });
    try {
      const response = await scoped.submit(last, actor);
      if (current(expected)) await finish(response, true);
    } catch (error) {
      if (current(expected)) fail(error, true);
    } finally {
      if (current(expected)) update({ busy: false });
    }
  }
  function edit() {
    const value = read();
    if (!alive || !admitted() || !['draft', 'review'].includes(value.mode) || value.last) return;
    generation++;
    update({
      mode: 'draft',
      prepared: null,
      acknowledged: false,
      retryAvailable: false,
      busy: false,
      error: '',
    });
  }
  function dispose() {
    alive = false;
    generation++;
  }
  return { prepare, submit, check, retry, edit, dispose };
}
