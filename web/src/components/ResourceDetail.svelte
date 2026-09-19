<script>
  import ResourceValues from './ResourceValues.svelte';
  import ResourceSources from './ResourceSources.svelte';
  import ResourceHistory from './ResourceHistory.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  import { stageLabels } from '../lib/case-stages.mjs';
  export let record,
    api,
    ondenied,
    onedit,
    onselect,
    canManage = false,
    disabled = false,
    historical = false;
  let history = false,
    historyBusy = false;
</script>

<section class="card fact-detail" aria-label="Detalle de recurso">
  <div class="section-heading">
    <h2>{record.values.title}</h2>
    <span class="badge" class:info={record.status === 'active'}
      >{record.status === 'active' ? 'Activo' : 'Archivado'}</span
    >
  </div>
  <p>
    Revisi&#243;n {record.revision} / {historical
      ? 'Consultada exactamente'
      : 'Actual al consultar'}
  </p>
  {#if historical}<p class="notice">
      Esta vista conserva la revisi&#243;n seleccionada. Puede existir una posterior.
    </p>
    <button class="secondary" disabled={disabled || historyBusy} onclick={() => onselect(record.id)}
      >Consultar recurso actual</button
    >{/if}
  <ResourceValues values={record.values} />
  {#if record.act}<h3>Acto de esta revisi&#243;n</h3>
    <ResourceValues values={record.act.values} act />
    <p>Revisi&#243;n del acto: {record.act.revision}</p>{/if}
  <ResourceSources sources={record.sources} act={record.act} />
  {#if record.reason}<p class="case-multiline">Motivo: {record.reason}</p>{/if}
  <details>
    <summary>Autor y contexto de esta captura</summary>
    <p>
      {record.recorded_by.email} / <time datetime={record.recorded_at}>{record.recorded_at}</time>
    </p>
    <FactAdministrativeCapture
      value={record.recorded_administration}
      label={'Administraci\u00f3n al registrar'}
    />
    <p>
      Etapa capturada: {record.recorded_stage.current
        ? stageLabels[record.recorded_stage.current.stage]
        : 'Sin etapa registrada'}
    </p>
    <p>Recurso: <code>{record.id}</code></p>
    {#if record.act}<p>Acto: <code>{record.act.id}</code></p>{/if}
    <p>Recibo: <code>{record.receipt.submission_digest}</code></p>
    <p>Captura: <code>{record.receipt.capture_digest}</code></p>
  </details>
  <div class="action-row">
    {#if canManage && !historical}
      {#if record.status === 'active'}
        <button
          class="primary"
          disabled={disabled || historyBusy}
          onclick={() => onedit('record_act')}>Registrar acto</button
        >
        <button
          class="secondary"
          disabled={disabled || historyBusy}
          onclick={() => onedit('correct')}>Corregir recurso</button
        >
        <button
          class="secondary"
          disabled={disabled || historyBusy}
          onclick={() => onedit('archive')}>Archivar recurso</button
        >
      {:else}<button
          class="primary"
          disabled={disabled || historyBusy}
          onclick={() => onedit('reactivate')}>Reactivar recurso</button
        >{/if}
    {/if}
    {#if canManage && record.act}<button
        class="secondary"
        disabled={disabled || historyBusy}
        onclick={() => onedit('correct_act')}>Corregir este acto</button
      >{/if}
    <button
      class="text-button"
      disabled={disabled || historyBusy}
      aria-expanded={history}
      onclick={() => (history = !history)}
      >{history ? 'Ocultar' : 'Ver'} historial de recurso</button
    >
  </div>
  {#if history}<ResourceHistory
      {api}
      id={record.id}
      {onselect}
      {ondenied}
      {disabled}
      bind:busy={historyBusy}
    />{/if}
</section>
