<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import ResourceEditorBody from './ResourceEditorBody.svelte';
  import { createResourceDraft } from '../lib/resource-draft.mjs';
  import { captureResourceDraft } from '../lib/resource-draft-values.mjs';
  import { refreshResourceReferences } from '../lib/resource-draft-references.mjs';
  import { caseState } from '../lib/case-state.mjs';
  import { resourceDraft, resourceCommand } from '../lib/procedural-resource-values.mjs';
  import {
    resourceFailure,
    resourceDenied,
    resourceUncertain,
    canResources,
  } from '../lib/procedural-resource-errors.mjs';
  import { readResourceSubmission } from '../lib/procedural-resource-submission.mjs';
  export let api,
    caseId,
    user,
    action,
    record = null,
    selectedAct = null,
    ondenied,
    onconfirmed,
    oncancel,
    disabled = false,
    pending = false,
    savedDraft = null;
  const administration = caseState(),
    scoped = api.caseResources(caseId),
    session = getContext('session-drafts');
  let id = record?.id || crypto.randomUUID();
  const isAct = ['record_act', 'correct_act'].includes(action);
  let actId = selectedAct?.id || crypto.randomUUID();
  let base = record,
    draft = resourceDraft(
      action === 'correct_act'
        ? selectedAct
          ? { act: selectedAct }
          : null
        : action === 'record_act'
          ? null
          : record,
      isAct,
    );
  let alive = true,
    busy = false,
    fieldsBusy = false,
    mode = 'draft',
    prepared = null,
    last = null,
    candidate = null,
    error = '';
  let blocked = true,
    restoredClosed = false,
    loaded = false,
    finished = false,
    inputs = null,
    view;
  const recovery = session
    ? createResourceDraft({
        session,
        caseId,
        capture: () =>
          captureResourceDraft({
            id,
            actId,
            selectedAct,
            action,
            base,
            draft,
            mode,
            last,
            inputs: (blocked ? null : view?.captureInputs()) ?? inputs,
          }),
        read: async () => {
          try {
            return base ? await scoped.get(base.id) : null;
          } catch (failure) {
            if ([403, 404].includes(failure.status)) failure.resourceOwner = true;
            throw failure;
          }
        },
      })
    : null;
  function admitted() {
    return alive && !finished && (!recovery || recovery.admitted());
  }
  function observe(context) {
    restoredClosed = context.closed;
    if (
      mode !== 'uncertain' &&
      ((context.current?.revision ?? 0) !== (base?.revision ?? 0) ||
        (context.current && (action === 'reactivate') !== (context.current.status === 'archived')))
    )
      mode = 'conflict';
  }
  async function initialize() {
    if (session?.canAdmit() && !canResources(session.principal()?.role, 'manage')) {
      deny({ status: 403, code: 'permission_denied' });
      return;
    }
    if (pending || !admitted()) return;
    if (!blocked) inputs = view?.captureInputs() ?? inputs;
    blocked = busy = true;
    error = '';
    try {
      let context;
      if (savedDraft && !loaded && recovery) {
        const result = await recovery.restore(savedDraft, (value, fresh) => {
          ({ id, actId, selectedAct, action, base, draft, mode, last, inputs } = value);
          loaded = true;
          context = fresh;
        });
        if (!admitted()) return;
        if (result.status !== 'restored') throw new Error('No se pudo recuperar el recurso.');
      } else {
        context = recovery
          ? await recovery.fresh()
          : { current: base, closed: $administration.closed };
        if (!context || !admitted()) return;
        if (!loaded) {
          recovery?.register(action, base, actId);
          loaded = true;
        }
      }
      observe(context);
      if (!context.closed && $administration.closed) {
        await $administration.refresh?.();
        if (!admitted()) return;
      }
      if (savedDraft && action === 'correct_act') {
        const exact = await scoped.revision(id, selectedAct.resourceRevision);
        if (!admitted()) return;
        if (exact.act?.id !== actId || exact.act.revision !== selectedAct.revision)
          throw new Error('El acto no coincide con su revision conservada.');
      }
      if (
        !savedDraft ||
        (await refreshResourceReferences(api, caseId, draft.values, isAct, admitted))
      )
        blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = resourceFailure(failure);
        if (failure.resourceSupportIndex !== undefined && [403, 404].includes(failure.status)) {
          supportDenied(
            failure,
            isAct ? ['evidence'] : ['resolution_evidence'],
            inputs?.supportRows?.[failure.resourceSupportIndex] ?? null,
          );
          if (admitted()) blocked = false;
        } else if (failure.resourceOwner || resourceDenied(failure)) deny(failure);
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
  function supportDenied(failure, path, rowId = null) {
    if (failure.code === 'case_not_found') return deny(failure);
    error = resourceFailure(failure);
    prepared = null;
    if (mode === 'review') mode = 'draft';
    recovery?.discardSupport(path, rowId);
    if (isAct) {
      if (!blocked) inputs = view?.captureInputs() ?? inputs;
      const index = inputs?.supportRows?.indexOf(rowId) ?? -1;
      if (index >= 0) draft.values.evidence[index] = null;
    } else draft.values.resolution_evidence = null;
  }
  function supportContext(path, rowId = null) {
    return blocked
      ? null
      : (recovery?.supportContext(
          path,
          rowId,
          () => !blocked && mode === 'draft' && !restoredClosed && admitted(),
          observe,
          deny,
        ) ?? null);
  }
  function discardSupport(path, rowId = null) {
    recovery?.discardSupport(path, rowId);
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
    mode === 'uncertain';
  function fail(failure, writing = false) {
    if (resourceDenied(failure)) {
      deny(failure);
      return;
    }
    error = resourceFailure(failure);
    if (failure.code === 'case_closed') restoredClosed = true;
    prepared = null;
    if (
      writing &&
      (resourceUncertain(failure) || failure.code === 'procedural_resource_operation_conflict')
    )
      mode = 'uncertain';
    else if (
      [
        'procedural_resource_revision_conflict',
        'procedural_resource_archived',
        'procedural_resource_state_unchanged',
      ].includes(failure.code)
    ) {
      mode = 'conflict';
      candidate = null;
    } else mode = 'draft';
  }
  async function prepare() {
    if (!admitted() || frozen || mode !== 'draft') return;
    inputs = view?.captureInputs() ?? inputs;
    busy = true;
    error = '';
    try {
      const hasValues = ['register', 'correct', 'record_act', 'correct_act'].includes(action);
      const hasReason = ['correct', 'correct_act', 'archive', 'reactivate'].includes(action);
      const command = resourceCommand({
        operation_id: crypto.randomUUID(),
        resource_id: id,
        change: {
          action,
          expected_revision: base?.revision || 0,
          ...(hasValues ? { values: draft.values } : {}),
          ...(hasReason ? { reason: draft.reason } : {}),
          ...(isAct ? { act_id: actId } : {}),
          ...(action === 'correct_act' ? { expected_act_revision: selectedAct.revision } : {}),
        },
      });
      const value = await scoped.prepare(command, { id: user.id, email: user.email });
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
      if (alive) error = 'El registro se guardo. Consulta de nuevo sin reenviar.';
    }
  }
  async function submit() {
    if (!admitted() || frozen || mode !== 'review' || !prepared) return;
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
      const result = await readResourceSubmission(scoped, last);
      if (!admitted()) return;
      if (result.state === 'matched') await finish(result.record, true);
      else if (result.state === 'absent')
        error =
          'La revision aun no esta disponible. El resultado sigue incierto; puedes consultar de nuevo.';
      else {
        mode = 'conflict';
        candidate = null;
        error = 'La revision corresponde a otro envio. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (admitted()) {
        error = resourceFailure(failure);
        if (resourceDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (pending || blocked || !admitted()) return;
    busy = true;
    error = '';
    try {
      const value = await scoped.get(id);
      if (admitted()) candidate = value;
    } catch (failure) {
      if (admitted()) {
        error = resourceFailure(failure);
        if (resourceDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (!admitted() || frozen || !candidate || action === 'register' || action === 'correct_act')
      return;
    if ((action === 'reactivate') !== (candidate.status === 'archived')) return;
    base = candidate;
    recovery?.register(action, base, actId);
    last = null;
    candidate = null;
    mode = 'draft';
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

<ResourceEditorBody
  {api}
  {caseId}
  {user}
  {action}
  bind:draft
  {mode}
  {prepared}
  {candidate}
  {error}
  {pending}
  {frozen}
  {blocked}
  {restoredClosed}
  {inputs}
  {supportContext}
  {discardSupport}
  {supportDenied}
  canApply={admitted}
  ondenied={deny}
  fieldsDisabled={disabled ||
    busy ||
    blocked ||
    restoredClosed ||
    $administration.closed ||
    mode === 'uncertain'}
  bind:fieldsBusy
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
