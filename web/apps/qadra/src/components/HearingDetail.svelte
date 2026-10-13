<script>
  import HearingValues from './HearingValues.svelte';
  import HearingHistory from './HearingHistory.svelte';
  import { hearingKinds, hearingStatus } from '../lib/hearings.mjs';
  import { stageLabels } from '../lib/case-stages.mjs';
  export let record,
    api,
    ondenied,
    onedit,
    oncurrent,
    canManage = false,
    disabled = false,
    historical = false;
  let history = false,
    historyBusy = false,
    historyView;
  export async function refreshHistory() {
    if (historyView) await historyView.refresh();
  }
</script>

<section class="card hearing-detail" aria-label="Detalle de audiencia">
  <div class="section-heading">
    <h2>{hearingKinds[record.values.kind]?.label}</h2>
    <span class="badge" class:info={record.status === 'scheduled'}
      >{hearingStatus[record.status]}</span
    >
  </div>
  <p class="hint">
    Audiencia <code>{record.id}</code> / Revisi&#243;n {record.revision}{historical
      ? ' consultada exactamente'
      : ' actual al consultar'}
  </p>
  {#if historical}<p class="notice">
      Esta vista conserva la revisi&#243;n seleccionada. Puede existir una posterior.
    </p>
    <button class="secondary" disabled={disabled || historyBusy} onclick={oncurrent}
      >Consultar registro actual</button
    >{/if}
  {#if record.reason}<p class="case-multiline">Motivo: {record.reason}</p>{/if}
  <HearingValues
    values={record.values}
    participants={record.participants}
    support={record.support}
  />
  <details class="hearing-provenance">
    <summary>Contexto y autor del registro</summary>
    <p>
      Etapa de programaci&#243;n: {stageLabels[record.scheduling_context.stage]} / Revisi&#243;n {record
        .scheduling_context.stage_revision}.
    </p>
    <p>
      Administraci&#243;n de programaci&#243;n: revisi&#243;n {record.scheduling_context
        .administration_revision}. Administraci&#243;n al registrar: revisi&#243;n {record.recorded_administration_revision}.
    </p>
    <p>
      {record.recorded_by.email} / <time datetime={record.recorded_at}>{record.recorded_at}</time>
    </p>
    <p class="hint">
      El contexto y los participantes son hist&#243;ricos; sus cambios posteriores no sustituyen
      esta evidencia.
    </p>
  </details>
  <div class="action-row">
    {#if canManage && record.status === 'scheduled' && !historical}
      <button class="primary" disabled={disabled || historyBusy} onclick={() => onedit('replace')}
        >Corregir o reprogramar</button
      >
      <button class="secondary" disabled={disabled || historyBusy} onclick={() => onedit('cancel')}
        >Cancelar audiencia</button
      >
    {/if}
    <button
      class="text-button"
      disabled={disabled || historyBusy}
      aria-expanded={history}
      onclick={() => (history = !history)}
      >{history ? 'Ocultar historial de audiencias' : 'Ver historial de audiencias'}</button
    >
  </div>
  {#if history}<HearingHistory
      {api}
      id={record.id}
      {ondenied}
      {disabled}
      bind:busy={historyBusy}
      bind:this={historyView}
    />{/if}
</section>
