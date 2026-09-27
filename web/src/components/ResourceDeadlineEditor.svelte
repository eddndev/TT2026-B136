<script>
  import { onDestroy } from 'svelte';
  import DeadlineFields from './DeadlineFields.svelte';
  import ResourceDeadlineContext from './ResourceDeadlineContext.svelte';
  import ResourceDeadlineReview from './ResourceDeadlineReview.svelte';
  import ResourceValues from './ResourceValues.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import {
    canDeadlines,
    deadlineFailure,
    deadlineDenied,
    deadlineUncertain,
  } from '../lib/deadline-errors.mjs';
  import { resourceActivityFailure } from '../lib/resource-activity-errors.mjs';
  import {
    initialDeadlinePolicies,
    deadlinePoliciesCommand,
  } from '../lib/deadline-editor-policies.mjs';
  import { readResourceDeadlineSubmission } from '../lib/resource-deadline-recovery.mjs';
  export let api,
    caseId,
    user,
    resource,
    head,
    ondenied,
    onconfirmed,
    oncancel,
    disabled = false,
    pending = false;
  const scoped = api.caseResourceDeadlines(caseId, resource.id),
    deadlines = api.deadlines(caseId),
    associations = api.caseResourceActivities(caseId, resource.id),
    resources = api.caseResources(caseId),
    administration = caseState();
  const deadlineId = crypto.randomUUID(),
    associationId = crypto.randomUUID();
  let base = head,
    selection = {
      resource: {
        id: resource.id,
        revision: resource.revision,
        capture_digest: resource.receipt.capture_digest,
      },
      act: null,
    };
  let definition = {
    title: '',
    profile: null,
    responsible_id: '',
    input: {
      selection: { case_id: caseId, source: { kind: '' }, qualification: null },
      calendar: null,
      ordered_quantity: null,
      qualification: {
        statement: '',
        locator: '',
        scope_applies: { kind: '' },
        unresolved_incident: { kind: '' },
        conditions: [],
      },
    },
  };
  let profile = null,
    responsible = null,
    policies = initialDeadlinePolicies(definition);
  let alive = true,
    busy = false,
    fieldsBusy = false,
    contextBusy = false,
    mode = 'draft',
    prepared = null,
    last = null,
    retryAvailable = false,
    paired = false,
    candidate = null,
    acknowledged = false,
    error = '';
  $: pending = busy || fieldsBusy || contextBusy;
  $: frozen =
    disabled ||
    pending ||
    $administration.closed ||
    !canDeadlines(user.role, 'manage') ||
    mode === 'uncertain' ||
    mode === 'confirmed';
  const message = (failure) =>
    failure?.code?.startsWith('resource_')
      ? resourceActivityFailure(failure)
      : deadlineFailure(failure);
  function fail(failure, writing = false) {
    if (deadlineDenied(failure)) {
      ondenied(failure);
      return;
    }
    error = message(failure);
    prepared = null;
    acknowledged = false;
    if (
      writing &&
      (deadlineUncertain(failure) ||
        [
          'resource_activity_operation_conflict',
          'deadline_operation_conflict',
          'deadline_revision_conflict',
        ].includes(failure.code))
    )
      mode = 'uncertain';
    else if (
      [
        'resource_activity_resource_revision_conflict',
        'resource_activity_resource_archived',
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
    acknowledged = false;
    try {
      const command = {
        case_id: caseId,
        resource_id: resource.id,
        association_id: associationId,
        expected_resource_revision: base.revision,
        ...structuredClone(selection),
        deadline: {
          operation_id: crypto.randomUUID(),
          deadline_id: deadlineId,
          change: {
            action: 'register',
            expected_revision: 0,
            definition: structuredClone(definition),
            tracking: deadlinePoliciesCommand(policies, definition),
          },
        },
      };
      const value = await scoped.prepare(command, {
        id: user.id,
        email: user.email,
        role: user.role,
      });
      if (alive) {
        prepared = structuredClone(value);
        mode = 'review';
      }
    } catch (failure) {
      if (alive) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function finish(value, exact = false) {
    prepared = null;
    mode = 'confirmed';
    try {
      await onconfirmed(value.association, exact);
    } catch {
      if (alive) error = 'El plazo y vinculo se guardaron. Consulta de nuevo sin reenviar.';
    }
  }
  async function submit() {
    if (frozen || mode !== 'review' || !prepared || !acknowledged) return;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    try {
      const value = await scoped.submit(last);
      if (alive) await finish(value);
    } catch (failure) {
      if (alive) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function check() {
    if (pending || disabled || !last) return;
    busy = true;
    error = '';
    retryAvailable = false;
    paired = false;
    try {
      const checked = await readResourceDeadlineSubmission(deadlines, associations, last);
      if (!alive) return;
      paired = checked.state === 'paired';
      retryAvailable = paired || checked.state === 'absent';
      error = paired
        ? 'Los recibos coinciden. Confirma el mismo envio para verificar que se guardaron juntos.'
        : retryAvailable
          ? 'Ninguna revision esta disponible todavia. La ausencia no confirma que el envio fallo.'
          : 'Los dos recibos no corresponden al mismo envio confirmado. Conserva el borrador y consulta de nuevo.';
    } catch (failure) {
      if (alive) {
        error = message(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function retry() {
    if (
      pending ||
      disabled ||
      ($administration.closed && !paired) ||
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
      if (alive) await finish(value, true);
    } catch (failure) {
      if (alive) {
        fail(failure, true);
        if (paired && failure.code === 'resource_activity_operation_conflict')
          error =
            'El servidor no confirma el origen conjunto de estos recibos. El resultado sigue incierto.';
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (pending || disabled) return;
    busy = true;
    error = '';
    candidate = null;
    try {
      const value = await resources.get(resource.id);
      if (alive) candidate = value;
    } catch (failure) {
      if (alive) {
        error = message(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (frozen || !candidate || candidate.status !== 'active') return;
    base = candidate;
    candidate = null;
    mode = 'draft';
    error = '';
  }
  onDestroy(() => {
    alive = false;
    pending = false;
    for (const scope of [scoped, deadlines, associations, resources]) scope.dispose();
  });
</script>

<section class="card case-editor fact-editor" aria-label="Formulario de plazo" aria-busy={pending}>
  <h2>Crear plazo desde el recurso</h2>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if mode === 'uncertain'}
    <h3>Resultado incierto</h3>
    <p>Consulta las revisiones exactas del plazo y del v&#237;nculo antes de decidir.</p>
    <button class="primary" disabled={pending || disabled} onclick={check}
      >Consultar resultado</button
    >
    <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    {#if retryAvailable}<button
        class="secondary"
        disabled={pending || disabled || ($administration.closed && !paired)}
        onclick={retry}
        >{paired ? 'Confirmar origen del env\u00edo' : 'Reintentar envio exacto'}</button
      >{/if}
  {:else if mode === 'conflict'}
    <h3>El registro cambi&#243;</h3>
    <p>La declaraci&#243;n del plazo y las capturas seleccionadas se conservan.</p>
    <button class="secondary" disabled={pending || disabled} onclick={compare}
      >Comparar con registro actual</button
    >
    {#if candidate}
      <p>
        Cabeza actual del recurso: revisi&#243;n {candidate.revision} / {candidate.status ===
        'active'
          ? 'Activo'
          : 'Archivado'}
      </p>
      <ResourceValues values={candidate.values} />
      <button class="primary" disabled={frozen || candidate.status !== 'active'} onclick={accept}
        >Usar base actual y conservar borrador</button
      >
    {/if}
  {:else if mode === 'review' && prepared}
    <ResourceDeadlineReview value={prepared} {profile} />
    <label class="checkbox"
      ><input type="checkbox" bind:checked={acknowledged} disabled={frozen} />Reconozco el resultado
      y las capturas seleccionadas</label
    >
    <div class="action-row">
      <button class="primary" disabled={frozen || !acknowledged} onclick={submit}
        >Confirmar plazo y vinculo</button
      >
      <button
        class="secondary"
        disabled={frozen}
        onclick={() => {
          mode = 'draft';
          prepared = null;
          acknowledged = false;
        }}>Volver al borrador</button
      >
    </div>
  {/if}
  {#if mode !== 'review' && mode !== 'confirmed'}
    <ResourceDeadlineContext
      {api}
      {caseId}
      {resource}
      {ondenied}
      bind:selection
      bind:busy={contextBusy}
      disabled={frozen || mode !== 'draft'}
    />
    <DeadlineFields
      {api}
      {caseId}
      {ondenied}
      bind:value={definition}
      bind:profile
      bind:responsible
      bind:policies
      bind:pending={fieldsBusy}
      disabled={frozen || mode !== 'draft'}
    />
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={prepare}
      >Preparar plazo y vinculo</button
    >
  {/if}
  <button class="text-button" disabled={pending || disabled} onclick={oncancel}
    >Cerrar formulario</button
  >
</section>
