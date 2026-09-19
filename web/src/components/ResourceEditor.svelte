<script>
  import { onDestroy } from 'svelte';
  import ResourceFields from './ResourceFields.svelte';
  import ResourceActFields from './ResourceActFields.svelte';
  import ResourceValues from './ResourceValues.svelte';
  import ResourceSources from './ResourceSources.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import {
    resourceDraft,
    resourceCommand,
    resourceActions,
  } from '../lib/procedural-resource-values.mjs';
  import {
    resourceFailure,
    resourceDenied,
    resourceUncertain,
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
    pending = false;
  const administration = caseState(),
    scoped = api.caseResources(caseId);
  const id = record?.id || crypto.randomUUID(),
    isAct = ['record_act', 'correct_act'].includes(action);
  const actId = selectedAct?.id || crypto.randomUUID();
  let base = record,
    draft = resourceDraft(
      action === 'correct_act' ? { act: selectedAct } : action === 'record_act' ? null : record,
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
  $: pending = busy || fieldsBusy;
  $: frozen = disabled || pending || $administration.closed || mode === 'uncertain';
  function fail(failure, writing = false) {
    if (resourceDenied(failure)) {
      ondenied(failure);
      return;
    }
    error = resourceFailure(failure);
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
    if (frozen || mode !== 'draft') return;
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
      await onconfirmed(value, exact);
    } catch {
      if (alive) error = 'El registro se guardo. Consulta de nuevo sin reenviar.';
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
    try {
      const result = await readResourceSubmission(scoped, last);
      if (!alive) return;
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
      if (alive) {
        error = resourceFailure(failure);
        if (resourceDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (pending) return;
    busy = true;
    error = '';
    try {
      const value = await scoped.get(id);
      if (alive) candidate = value;
    } catch (failure) {
      if (alive) {
        error = resourceFailure(failure);
        if (resourceDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (frozen || !candidate || action === 'register' || action === 'correct_act') return;
    if ((action === 'reactivate') !== (candidate.status === 'archived')) return;
    base = candidate;
    candidate = null;
    mode = 'draft';
    error = '';
  }
  onDestroy(() => {
    alive = false;
    pending = false;
    scoped.dispose();
  });
</script>

<section
  class="card case-editor fact-editor"
  aria-label="Formulario de recurso"
  aria-busy={pending}
>
  <h2>{resourceActions[action]}</h2>
  <p class="hint">
    Conserva lo declarado y sus fuentes. El registro no determina efectos jur&#237;dicos ni inicia
    plazos.
  </p>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if mode === 'uncertain'}
    <div class="case-comparison">
      <h3>Resultado incierto</h3>
      <p>Consulta el recibo del env&#237;o. Una ausencia temporal no confirma que fall&#243;.</p>
      <button class="primary" disabled={pending || disabled} onclick={check}
        >Consultar envio exacto</button
      >
    </div>
  {:else if mode === 'conflict'}
    <div class="case-comparison">
      <h3>El registro cambi&#243;</h3>
      <p>Tu borrador se conserva. Consulta la base actual antes de decidir.</p>
      <button class="secondary" disabled={pending || disabled} onclick={compare}
        >Comparar con registro actual</button
      >
      {#if candidate}<p>
          Revisi&#243;n {candidate.revision} / {candidate.status === 'active'
            ? 'Activo'
            : 'Archivado'}
        </p>
        <ResourceValues values={candidate.values} />
        {#if action === 'correct_act'}<p>
            Consulta en la historia la &#250;ltima revisi&#243;n del acto y abre su correcci&#243;n.
            Este borrador permanece visible hasta cerrar el formulario.
          </p>
        {:else}<button
            class="primary"
            disabled={frozen ||
              action === 'register' ||
              (action === 'reactivate') !== (candidate.status === 'archived')}
            onclick={accept}>Usar base actual y conservar borrador</button
          >{/if}
      {/if}
    </div>
  {:else if mode === 'review' && prepared}
    <h3>Revisar antes de confirmar</h3>
    <ResourceValues values={prepared.values} />
    {#if prepared.act}<h3>Acto declarado</h3>
      <ResourceValues values={prepared.act.values} act />{/if}
    <ResourceSources sources={prepared.sources} act={prepared.act} />
    {#if draft.reason}<p class="case-multiline">Motivo: {draft.reason}</p>{/if}
    <div class="action-row">
      <button class="primary" disabled={frozen} onclick={submit}>Confirmar registro</button>
      <button
        class="secondary"
        disabled={pending || disabled}
        onclick={() => {
          prepared = null;
          mode = 'draft';
        }}>Volver al borrador</button
      >
    </div>
  {/if}
  {#if mode !== 'review' && mode !== 'confirmed'}
    <fieldset disabled={frozen || mode !== 'draft'}>
      {#if isAct}<ResourceActFields
          {api}
          {caseId}
          {user}
          bind:values={draft.values}
          {ondenied}
          disabled={frozen || mode !== 'draft'}
          bind:busy={fieldsBusy}
        />
      {:else if ['register', 'correct'].includes(action)}<ResourceFields
          {api}
          {caseId}
          {user}
          bind:values={draft.values}
          {ondenied}
          disabled={frozen || mode !== 'draft'}
          bind:busy={fieldsBusy}
        />
      {:else}<p>El archivo es organizativo. No declara desistimiento ni modifica actos previos.</p>
        <ResourceValues values={draft.values} />{/if}
      {#if ['correct', 'correct_act', 'archive', 'reactivate'].includes(action)}<label
          >Motivo<textarea bind:value={draft.reason} maxlength="1000" rows="3"></textarea></label
        >{/if}
    </fieldset>
    {#if mode === 'draft'}<button class="primary" disabled={frozen} onclick={prepare}
        >Preparar registro</button
      >{/if}
  {/if}
  <button class="text-button" disabled={pending || disabled} onclick={oncancel}
    >Cerrar formulario</button
  >
</section>
