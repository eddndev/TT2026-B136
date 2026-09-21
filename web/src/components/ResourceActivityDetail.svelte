<script>
  import ResourceActivitySources from './ResourceActivitySources.svelte';
  import ResourceActivityTarget from './ResourceActivityTarget.svelte';
  import ResourceActivityHistory from './ResourceActivityHistory.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  import { resourceActivityStatuses } from '../lib/resource-activity-values.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  export let view,
    api,
    ondenied,
    onselect,
    onedit,
    canManage = false,
    disabled = false,
    historical = false;
  let history = false,
    historyBusy = false;
  $: record = view.association;
</script>

<section class="card fact-detail" aria-label="Detalle de actividad vinculada">
  <div class="section-heading">
    <h2>Actividad del recurso</h2>
    <span class="badge" class:info={record.status === 'linked'}
      >{resourceActivityStatuses[record.status]}</span
    >
  </div>
  <p>
    V&#237;nculo / Revisi&#243;n {record.revision} / {historical
      ? 'Consulta historica exacta'
      : 'Actual al consultar'}
  </p>
  {#if historical}<button
      class="secondary"
      disabled={disabled || historyBusy}
      onclick={() => onselect(record.id)}>Consultar v&#237;nculo actual</button
    >{/if}
  {#if record.reason}<p class="case-multiline">Motivo: {record.reason}</p>{/if}
  <ResourceActivitySources sources={record.sources} />
  <section class="case-comparison" aria-label="Actividad actual">
    <h3>Actividad actual</h3>
    <p>Consultada el {deadlineInstantLabel(view.checked_at)}</p>
    <ResourceActivityTarget target={view.current_target} historical={false} />
  </section>
  <details>
    <summary>Autor y contexto del v&#237;nculo</summary>
    <p>{record.recorded_by.email} / {record.recorded_at}</p>
    <FactAdministrativeCapture
      value={record.recorded_administration}
      label={'Administracion al vincular'}
    />
    <p>Cabeza del recurso al confirmar: revisi&#243;n {record.recorded_resource_head.revision}</p>
    <p>Identidad: <code>{record.id}</code></p>
    <p>Recibo: <code>{record.receipt.submission_digest}</code></p>
    <p>Captura: <code>{record.receipt.capture_digest}</code></p>
  </details>
  <div class="action-row">
    {#if canManage && !historical && record.status === 'linked'}<button
        class="secondary"
        disabled={disabled || historyBusy}
        onclick={() => onedit(record)}>Desvincular actividad</button
      >{/if}
    <button
      class="text-button"
      disabled={disabled || historyBusy}
      aria-expanded={history}
      onclick={() => (history = !history)}
      >{history ? 'Ocultar historial de v\u00ednculo' : 'Ver historial de v\u00ednculo'}</button
    >
  </div>
  {#if history}<ResourceActivityHistory
      {api}
      id={record.id}
      {onselect}
      {ondenied}
      {disabled}
      bind:busy={historyBusy}
    />{/if}
</section>
