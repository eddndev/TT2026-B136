<script>
  import HearingResultSources from './HearingResultSources.svelte';
  import HearingResultValues from './HearingResultValues.svelte';
  import HearingResultHistory from './HearingResultHistory.svelte';
  import { resultStatus } from '../lib/hearing-result-errors.mjs';
  export let record,
    api,
    ondenied,
    onedit,
    oncontinue,
    onprevious,
    onselect,
    canManage = false,
    disabled = false,
    historical = false;
  let history = false,
    historyBusy = false;
</script>

<section class="card hearing-result-detail" aria-label="Detalle del resultado declarado">
  <div class="section-heading">
    <h2>Sesi&#243;n o acto declarado</h2>
    <span class="badge" class:info={record.status === 'recorded'}
      >{resultStatus[record.status]}</span
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
      >Consultar resultado actual</button
    >{/if}
  <HearingResultSources
    anchor={record.anchor}
    continuation={record.continuation}
    {onprevious}
    disabled={disabled || historyBusy}
  />
  <HearingResultValues
    values={record.values}
    attendees={record.attendees}
    support={record.support}
  />
  {#if record.reason}<p class="case-multiline">Motivo del registro: {record.reason}</p>{/if}
  <details>
    <summary>Autor y referencia de esta captura</summary>
    <p><code>{record.id}</code> / Revisi&#243;n {record.revision}</p>
    <p>
      {record.recorded_by.email} / <time datetime={record.recorded_at}>{record.recorded_at}</time>
    </p>
    <p>Administraci&#243;n al registrar: revisi&#243;n {record.recorded_administration_revision}</p>
    <p>Huella de valores <code>{record.values_digest}</code></p>
    <p>Recibo <code>{record.receipt.submission_digest}</code></p>
  </details>
  <div class="action-row">
    {#if canManage && !historical && record.status === 'recorded'}<button
        class="primary"
        disabled={disabled || historyBusy}
        onclick={() => onedit('correct')}>Rectificar registro</button
      ><button
        class="secondary"
        disabled={disabled || historyBusy}
        onclick={() => onedit('withdraw')}>Retirar registro</button
      >{/if}
    {#if canManage}<button class="secondary" disabled={disabled || historyBusy} onclick={oncontinue}
        >Registrar continuaci&#243;n</button
      >{/if}
    <button
      class="text-button"
      disabled={disabled || historyBusy}
      aria-expanded={history}
      onclick={() => (history = !history)}
      >{history ? 'Ocultar historial del registro' : 'Ver historial del registro'}</button
    >
  </div>
  {#if history}<HearingResultHistory
      {api}
      id={record.id}
      {ondenied}
      {onselect}
      {disabled}
      bind:busy={historyBusy}
    />{/if}
</section>
