<script>
  import { onDestroy } from 'svelte';
  import ResourceActivityFields from './ResourceActivityFields.svelte';
  import ResourceActivitySources from './ResourceActivitySources.svelte';
  import ResourceValues from './ResourceValues.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
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
    pending = false;
  const administration = caseState(),
    scoped = api.caseResourceActivities(caseId, resource.id),
    resources = api.caseResources(caseId);
  const action = record ? 'unlink' : 'link',
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
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled || pending || $administration.closed || mode === 'uncertain' || mode === 'confirmed';
  function fail(failure, writing = false) {
    if (resourceDenied(failure)) {
      ondenied(failure);
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
  async function finish(value, exact = false, view = null) {
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
    if (pending || !last) return;
    busy = true;
    error = '';
    retryAvailable = false;
    try {
      const value = await scoped.revision(id, last.result_revision);
      if (!alive) return;
      if (resourceActivityMatches(value.association, last))
        await finish(value.association, true, value);
      else {
        mode = 'conflict';
        candidate = null;
        error = 'La revision corresponde a otro envio. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (alive) {
        retryAvailable = failure.status === 404 && failure.code === 'resource_activity_not_found';
        error =
          failure.status === 404 && failure.code === 'resource_activity_not_found'
            ? 'La revision aun no esta disponible. El resultado sigue incierto; puedes consultar de nuevo.'
            : resourceActivityFailure(failure);
        if (resourceDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function retry() {
    if (
      pending ||
      disabled ||
      $administration.closed ||
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
      if (alive) await finish(value);
    } catch (failure) {
      if (alive) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (pending) return;
    busy = true;
    error = '';
    candidate = null;
    try {
      const current = await resources.get(resource.id);
      const currentAssociation = action === 'unlink' ? (await scoped.get(id)).association : null;
      if (alive) candidate = { resource: current, association: currentAssociation };
    } catch (failure) {
      if (alive) {
        error = resourceActivityFailure(failure);
        if (resourceDenied(failure)) ondenied(failure);
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
    base = candidate.resource;
    association = candidate.association;
    candidate = null;
    mode = 'draft';
    error = '';
  }
  onDestroy(() => {
    alive = false;
    pending = false;
    scoped.dispose();
    resources.dispose();
  });
</script>

<section
  class="card case-editor fact-editor"
  aria-label="Formulario de actividad vinculada"
  aria-busy={pending}
>
  <h2>{action === 'link' ? 'Vincular actividad existente' : 'Desvincular actividad'}</h2>
  <p class="hint">
    Organiza el recurso y conserva la evidencia exacta. No crea ni cancela audiencias, plazos o
    alertas.
  </p>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if mode === 'uncertain'}
    <h3>Resultado incierto</h3>
    <p>
      Consulta el recibo exacto antes de decidir. Una ausencia temporal no confirma que el
      env&#237;o fall&#243;.
    </p>
    <button class="primary" disabled={pending || disabled} onclick={check}
      >Consultar resultado</button
    >
    {#if retryAvailable}
      <p>
        La consulta no encontr&#243; la revisi&#243;n. Puedes repetir el mismo env&#237;o con su
        recibo, sin cambiar el borrador.
      </p>
      <button
        class="secondary"
        disabled={pending || disabled || $administration.closed}
        onclick={retry}>Reintentar envio exacto</button
      >
    {/if}
  {:else if mode === 'conflict'}
    <h3>El registro cambi&#243;</h3>
    <p>Tu selecci&#243;n exacta y el motivo se conservan.</p>
    <button class="secondary" disabled={pending || disabled} onclick={compare}
      >Comparar con registro actual</button
    >
    {#if candidate}
      <p>
        Cabeza actual del recurso: revisi&#243;n {candidate.resource.revision} / {candidate.resource
          .status === 'active'
          ? 'Activo'
          : 'Archivado'}
      </p>
      <ResourceValues values={candidate.resource.values} />
      {#if candidate.association}<p>
          V&#237;nculo: {candidate.association.status === 'linked' ? 'Vinculado' : 'Desvinculado'} / Revisi&#243;n
          {candidate.association.revision}
        </p>{/if}
      <button
        class="primary"
        disabled={frozen ||
          (action === 'link'
            ? candidate.resource.status !== 'active'
            : candidate.association.status !== 'linked')}
        onclick={accept}>Usar base actual y conservar borrador</button
      >
    {/if}
  {:else if mode === 'review' && prepared}
    <h3>Revisar antes de confirmar</h3>
    <p>Cabeza del recurso al preparar: revisi&#243;n {prepared.observed_resource_head.revision}</p>
    <ResourceActivitySources sources={prepared.sources} />
    {#if action === 'unlink'}<p class="case-multiline">
        Motivo: {prepared.command.change.reason}
      </p>{/if}
    <p>Autor: {prepared.recorded_by.email}</p>
    <div class="action-row">
      <button class="primary" disabled={frozen} onclick={submit}
        >{action === 'link' ? 'Confirmar v\u00ednculo' : 'Confirmar desvinculaci\u00f3n'}</button
      >
      <button
        class="secondary"
        disabled={frozen}
        onclick={() => {
          mode = 'draft';
          prepared = null;
        }}>Volver al borrador</button
      >
    </div>
  {/if}
  {#if mode !== 'review' && mode !== 'confirmed'}
    {#if action === 'link'}
      <ResourceActivityFields
        {api}
        {caseId}
        {resource}
        bind:selection
        {ondenied}
        disabled={frozen || mode !== 'draft'}
        bind:busy={fieldsBusy}
      />
    {:else}
      <label
        >Motivo<textarea
          bind:value={reason}
          maxlength="2000"
          disabled={frozen || mode !== 'draft'}
        /></label
      >
      <p>La captura vinculada permanecer&#225; en la historia.</p>
    {/if}
    <button
      class="primary"
      disabled={frozen ||
        mode !== 'draft' ||
        (action === 'link' ? !selection.target : !reason.trim())}
      onclick={prepare}
      >{action === 'link' ? 'Preparar v\u00ednculo' : 'Preparar desvinculaci\u00f3n'}</button
    >
  {/if}
  <button class="text-button" disabled={pending || disabled} onclick={oncancel}
    >Cerrar formulario</button
  >
</section>
