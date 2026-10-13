<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import ResourceActivityEditorBody from './ResourceActivityEditorBody.svelte';
  import {
    createActivityDraft,
    captureActivityDraft,
    refreshActivityReferences,
  } from '../lib/resource-activity-draft.mjs';
  import { canResources } from '../lib/procedural-resource-errors.mjs';
  import { caseState } from '../lib/case-state.mjs';
  import { resourceActivityCommand } from '../lib/resource-activity-values.mjs';
  import { resourceActivityMatches } from '../lib/resource-activity-validation.mjs';
  import {
    resourceActivityFailure,
    resourceDenied,
    resourceUncertain,
  } from '../lib/resource-activity-errors.mjs';
  export let api,
    caseId,
    user,
    resource,
    head,
    record = null,
    ondenied,
    onconfirmed,
    oncancel,
    disabled = false,
    pending = false,
    savedDraft = null;
  const session = getContext('session-drafts'),
    administration = caseState(),
    scoped = api.caseResourceActivities(caseId, resource.id),
    resources = api.caseResources(caseId);
  let action = record ? 'unlink' : 'link',
    id = record?.id || crypto.randomUUID();
  let base = head,
    association = record,
    selection = {
      resource: {
        id: resource.id,
        revision: resource.revision,
        capture_digest: resource.receipt.capture_digest,
      },
      act: null,
      target: null,
    };
  let reason = '',
    alive = true,
    busy = false,
    fieldsBusy = false,
    mode = 'draft',
    prepared = null,
    last = null,
    retryAvailable = false,
    candidate = null,
    error = '';
  let blocked = true,
    loaded = false,
    finished = false,
    restoredClosed = false,
    inputs = null,
    view;
  const recovery = session
    ? createActivityDraft({
        session,
        caseId,
        resourceId: resource.id,
        capture: () =>
          captureActivityDraft({
            id,
            action,
            base,
            association,
            selection,
            reason,
            mode,
            last,
            inputs: (blocked ? null : view?.captureInputs()) ?? inputs,
          }),
      })
    : null;
  function admitted() {
    return alive && !finished && (!recovery || recovery.admitted());
  }
  async function fresh() {
    if (!admitted()) return null;
    const value = session
      ? await session.authorizeCase(caseId)
      : { id: caseId, administration: $administration };
    if (!admitted()) return null;
    if (
      value.id !== caseId ||
      !['active', 'closed'].includes(value.administration.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    const current = await resources.get(resource.id);
    const currentAssociation = action === 'unlink' ? (await scoped.get(id)).association : null;
    return admitted()
      ? {
          current,
          currentAssociation,
          closed: value.administration.administrative_status === 'closed',
        }
      : null;
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
    retryAvailable = false;
    try {
      let context;
      if (savedDraft && !loaded && recovery) {
        const result = await recovery.restore(savedDraft, fresh, (value, current) => {
          ({ id, action, base, association, selection, reason, mode, last, inputs } = value);
          context = current;
          loaded = true;
        });
        if (!admitted()) return;
        if (result.status !== 'restored') throw new Error('No se pudo recuperar la actividad.');
      } else {
        context = await fresh();
        if (!context || !admitted()) return;
        if (!loaded) {
          recovery?.register(action, id, base.revision);
          loaded = true;
        }
      }
      restoredClosed = context.closed;
      if (
        mode !== 'uncertain' &&
        (context.current.revision !== base.revision ||
          (action === 'link'
            ? context.current.status !== 'active'
            : context.currentAssociation?.revision !== association.revision ||
              context.currentAssociation?.status !== 'linked'))
      )
        mode = 'conflict';
      if (!context.closed && $administration.closed) {
        await $administration.refresh?.();
        if (!admitted()) return;
      }
      if (
        savedDraft &&
        !(await refreshActivityReferences(api, caseId, resource.id, selection, admitted))
      )
        return;
      if (admitted()) blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = resourceActivityFailure(failure);
        if (resourceDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function deny(failure) {
    if (failure.code === 'case_not_found' || failure.status === 403)
      session?.registry.denyContext(caseId);
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = true;
    blocked = true;
    ondenied(failure);
  }
  function close() {
    if (pending) return;
    recovery?.close();
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    finished = true;
    oncancel();
  }
  $: closed = restoredClosed || $administration.closed;
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled ||
    blocked ||
    !admitted() ||
    pending ||
    closed ||
    mode === 'uncertain' ||
    mode === 'confirmed';
  function fail(failure, writing = false) {
    if (resourceDenied(failure)) {
      deny(failure);
      return;
    }
    error = resourceActivityFailure(failure);
    prepared = null;
    if (
      writing &&
      (resourceUncertain(failure) || failure.code === 'resource_activity_operation_conflict')
    )
      mode = 'uncertain';
    else if (
      [
        'resource_activity_revision_conflict',
        'resource_activity_resource_revision_conflict',
        'resource_activity_resource_archived',
        'resource_activity_state_unchanged',
      ].includes(failure.code)
    ) {
      mode = 'conflict';
      candidate = null;
    } else mode = 'draft';
  }
  async function prepare() {
    if (frozen || mode !== 'draft') return;
    busy = true;
    error = '';
    try {
      const command = resourceActivityCommand({
        case_id: caseId,
        resource_id: resource.id,
        association_id: id,
        operation_id: crypto.randomUUID(),
        expected_resource_revision: base.revision,
        change:
          action === 'link'
            ? { action, expected_revision: 0, ...structuredClone(selection) }
            : { action, expected_revision: association.revision, reason },
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
  async function finish(value, exact = false, view = null) {
    recovery?.close();
    finished = true;
    prepared = null;
    mode = 'confirmed';
    try {
      await onconfirmed(value, exact, view);
    } catch {
      if (alive) error = 'El vinculo se guardo. Consulta de nuevo sin reenviar.';
    }
  }
  async function submit() {
    if (frozen || mode !== 'review' || !prepared) return;
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
    retryAvailable = false;
    try {
      const value = await scoped.revision(id, last.result_revision);
      if (!admitted()) return;
      if (resourceActivityMatches(value.association, last))
        await finish(value.association, true, value);
      else {
        mode = 'conflict';
        candidate = null;
        error = 'La revision corresponde a otro envio. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (admitted()) {
        retryAvailable = failure.status === 404 && failure.code === 'resource_activity_not_found';
        error =
          failure.status === 404 && failure.code === 'resource_activity_not_found'
            ? 'La revision aun no esta disponible. El resultado sigue incierto; puedes consultar de nuevo.'
            : resourceActivityFailure(failure);
        if (resourceDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function retry() {
    if (
      pending ||
      disabled ||
      closed ||
      blocked ||
      !admitted() ||
      mode !== 'uncertain' ||
      !last ||
      !retryAvailable
    )
      return;
    busy = true;
    error = '';
    retryAvailable = false;
    try {
      const value = await scoped.submit(last);
      if (admitted()) await finish(value);
    } catch (failure) {
      if (admitted()) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (pending || blocked || !admitted()) return;
    busy = true;
    error = '';
    candidate = null;
    try {
      const current = await resources.get(resource.id);
      const currentAssociation = action === 'unlink' ? (await scoped.get(id)).association : null;
      if (admitted()) candidate = { resource: current, association: currentAssociation };
    } catch (failure) {
      if (admitted()) {
        error = resourceActivityFailure(failure);
        if (resourceDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (
      frozen ||
      !candidate ||
      (action === 'link'
        ? candidate.resource.status !== 'active'
        : candidate.association.status !== 'linked')
    )
      return;
    recovery?.register(action, id, candidate.resource.revision);
    base = candidate.resource;
    association = candidate.association;
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
    resources.dispose();
  });
</script>

<ResourceActivityEditorBody
  bind:this={view}
  {api}
  {caseId}
  {resource}
  bind:selection
  ondenied={deny}
  bind:fieldsBusy
  {action}
  bind:reason
  {pending}
  {disabled}
  {frozen}
  bind:mode
  bind:prepared
  {last}
  {candidate}
  {retryAvailable}
  {error}
  {closed}
  {blocked}
  {inputs}
  {check}
  {retry}
  {compare}
  {accept}
  {submit}
  {prepare}
  {close}
  {initialize}
/>
