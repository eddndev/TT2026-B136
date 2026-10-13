<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import FactEditorBody from './FactEditorBody.svelte';
  import { createFactDraft } from '../lib/fact-draft.mjs';
  import { factDraftFailure } from '../lib/fact-draft-failure.mjs';
  import { captureFactDraft } from '../lib/fact-draft-values.mjs';
  import { readFactOwner, refreshFactReferences } from '../lib/fact-draft-references.mjs';
  import { caseState } from '../lib/case-state.mjs';
  import { factDraft, factCommand } from '../lib/procedural-fact-values.mjs';
  import { factFailure, factDenied } from '../lib/procedural-fact-errors.mjs';
  import { readFactSubmission } from '../lib/procedural-fact-submission.mjs';
  export let api,
    caseId,
    user,
    family,
    resolution = null,
    action,
    record = null,
    ondenied,
    onconfirmed,
    oncancel,
    disabled = false,
    pending = false,
    savedDraft = null;
  const administration = caseState(),
    session = getContext('session-drafts');
  const parentId = resolution?.id ?? null;
  let id = record?.id || crypto.randomUUID();
  const scoped =
    family === 'resolution'
      ? api.caseResolutions(caseId)
      : api.caseNotifications(caseId, resolution.id);
  let base = record,
    draft = factDraft(family, record, resolution),
    alive = true,
    busy = false,
    fieldsBusy = false;
  let mode = 'draft',
    prepared = null,
    last = null,
    candidate = null,
    compared = false,
    error = '';
  let blocked = true,
    restoredClosed = false,
    loaded = false,
    finished = false,
    inputs = null,
    view;
  const recovery = session
    ? createFactDraft({
        session,
        caseId,
        family,
        parentId,
        capture: () =>
          captureFactDraft({
            id,
            family,
            parentId,
            action,
            base,
            draft,
            mode,
            last,
            inputs: (blocked ? null : view?.captureInputs()) ?? inputs,
          }),
        read: () => readFactOwner(api, scoped, caseId, parentId, base, admitted),
      })
    : null;
  function admitted() {
    return alive && !finished && (!recovery || recovery.admitted());
  }
  function observe(value) {
    restoredClosed = value.closed;
    if (mode !== 'uncertain' && (value.current?.revision ?? 0) !== (base?.revision ?? 0))
      mode = 'conflict';
  }
  async function initialize() {
    if (pending || !admitted()) return;
    if (!blocked) inputs = view?.captureInputs() ?? inputs;
    blocked = busy = true;
    error = '';
    try {
      let fresh;
      if (savedDraft && !loaded && recovery) {
        const result = await recovery.restore(savedDraft, (value, context) => {
          ({ id, action, base, draft, mode, last, inputs } = value);
          loaded = true;
          fresh = context;
        });
        if (!admitted()) return;
        if (result.status !== 'restored') throw new Error('No se pudo recuperar la declaracion.');
      } else {
        fresh = recovery
          ? await recovery.fresh()
          : { current: base, closed: $administration.closed };
        if (!fresh || !admitted()) return;
        if (!loaded) {
          recovery?.register(action, base?.id ?? null, base?.revision ?? 0);
          loaded = true;
        }
      }
      observe(fresh);
      if (!fresh.closed && $administration.closed) {
        await $administration.refresh?.();
        if (!admitted()) return;
      }
      if (!savedDraft || (await refreshFactReferences(api, caseId, draft.values, admitted)))
        blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = factFailure(failure);
        if (failure.draftSupport && [403, 404].includes(failure.status))
          supportDenied(failure, failure.draftSupport);
        else if (failure.draftOwner || factDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function deny(failure) {
    recovery?.deny(failure);
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    ondenied(failure);
  }
  function supportDenied(failure, path) {
    if (failure.code === 'case_not_found') return deny(failure);
    error = factFailure(failure);
    prepared = null;
    if (mode === 'review') mode = 'draft';
    recovery?.discardSupport(path);
    if (path[0] === 'representation') draft.values.representation.provenance.support = null;
    else draft.values.provenance.support = null;
  }
  function supportContext(path) {
    return blocked ? null : (recovery?.supportContext(path, () => !blocked, observe, deny) ?? null);
  }
  function discardSupport(path) {
    recovery?.discardSupport(path);
  }
  function close() {
    if (pending) return;
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    recovery?.close();
    finished = true;
    oncancel();
  }
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled ||
    pending ||
    blocked ||
    restoredClosed ||
    !admitted() ||
    $administration.closed ||
    ['uncertain', 'exhausted'].includes(mode);
  function fail(failure, writing = false) {
    if (factDenied(failure)) {
      deny(failure);
      return;
    }
    ({ error, mode } = factDraftFailure(failure, writing));
    if (failure.code === 'case_closed') restoredClosed = true;
    prepared = null;
    if (mode === 'conflict') compared = false;
  }

  async function prepare() {
    if (!admitted() || frozen || mode !== 'draft') return;
    inputs = view?.captureInputs() ?? inputs;
    busy = true;
    error = '';
    try {
      const command = factCommand(draft, {
        family,
        action,
        base,
        operationId: crypto.randomUUID(),
        id,
        resolutionId: resolution?.id,
      });
      const value = await scoped.prepare(command, user.id);
      if (admitted()) {
        prepared = structuredClone(value);
        mode = 'review';
      }
    } catch (failure) {
      if (admitted()) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function finish(value, exact = false) {
    recovery?.close();
    finished = true;
    prepared = null;
    mode = 'confirmed';
    try {
      await onconfirmed(value, exact);
    } catch {
      if (alive)
        error =
          'El registro se guard\u00f3. Vuelve a consultar sin reenviar si faltan datos en pantalla.';
    }
  }
  async function submit() {
    if (!admitted() || frozen || !prepared || mode !== 'review') return;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    mode = 'uncertain';
    try {
      const value = await scoped.submit(last);
      if (admitted()) await finish(value);
    } catch (failure) {
      if (admitted()) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function check() {
    if (pending || blocked || !admitted() || !last) return;
    busy = true;
    error = '';
    try {
      const value = await readFactSubmission(scoped, last);
      if (!admitted()) return;
      if (value.state === 'matched') await finish(value.record, true);
      else if (value.state === 'absent')
        error =
          'La revisi\u00f3n a\u00fan no est\u00e1 disponible. El resultado sigue incierto; puedes consultar de nuevo.';
      else {
        mode = 'conflict';
        candidate = value.record;
        compared = false;
        error = 'La revisi\u00f3n corresponde a otro env\u00edo. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (admitted()) {
        error = factFailure(failure);
        if (factDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (pending || blocked || !admitted()) return;
    busy = true;
    error = '';
    compared = false;
    try {
      const value = await scoped.get(id);
      if (admitted()) {
        candidate = value;
        compared = true;
      }
    } catch (failure) {
      if (admitted()) {
        error = factFailure(failure);
        if (factDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (
      !admitted() ||
      !compared ||
      !candidate ||
      candidate.status !== 'recorded' ||
      action === 'record' ||
      frozen
    )
      return;
    base = candidate;
    recovery?.register(action, base.id, base.revision);
    mode = 'draft';
    compared = false;
    error = '';
  }
  onMount(initialize);
  onDestroy(() => {
    recovery?.dispose();
    alive = false;
    pending = false;
    scoped.dispose();
  });
</script>

<FactEditorBody
  {api}
  {caseId}
  {family}
  {resolution}
  {action}
  {base}
  bind:draft
  {mode}
  {prepared}
  {last}
  {candidate}
  {compared}
  {error}
  {pending}
  {frozen}
  fieldsDisabled={disabled ||
    busy ||
    blocked ||
    restoredClosed ||
    $administration.closed ||
    ['uncertain', 'exhausted'].includes(mode)}
  bind:fieldsBusy
  {blocked}
  {restoredClosed}
  {inputs}
  {supportContext}
  {discardSupport}
  {supportDenied}
  canApply={admitted}
  ondenied={deny}
  {prepare}
  {submit}
  {check}
  {compare}
  {accept}
  {close}
  retry={initialize}
  back={() => {
    prepared = null;
    mode = 'draft';
  }}
  bind:this={view}
/>
