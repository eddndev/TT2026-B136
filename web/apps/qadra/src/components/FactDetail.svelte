<script>
  import FactValues from './FactValues.svelte';
  import FactSources from './FactSources.svelte';
  import FactHistory from './FactHistory.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  export let record,
    family,
    api,
    ondenied,
    onedit,
    onselect,
    canManage = false,
    disabled = false,
    historical = false;
  const singular = family === 'resolution' ? 'resoluci\u00f3n' : 'notificaci\u00f3n';
  let history = false,
    historyBusy = false;
</script>

<section class="card fact-detail" aria-label={`Detalle de ${singular}`}>
  <div class="section-heading">
    <h2>
      {family === 'resolution' ? 'Resoluci\u00f3n declarada' : 'Pr\u00e1ctica de notificaci\u00f3n'}
    </h2>
    <span class="badge" class:info={record.status === 'recorded'}
      >{record.status === 'recorded' ? 'Registrado' : 'Retirado'}</span
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
      >Consultar {singular} actual</button
    >{/if}
  <FactValues values={record.values} {family} /><FactSources sources={record.sources} />
  {#if record.reason}<p class="case-multiline">Motivo: {record.reason}</p>{/if}
  <details>
    <summary>Autor y referencia de esta captura</summary>
    <p><code>{record.id}</code> / Revisi&#243;n {record.revision}</p>
    <p>
      {record.recorded_by.email} / <time datetime={record.recorded_at}>{record.recorded_at}</time>
    </p>
    <FactAdministrativeCapture
      value={record.recorded_administration}
      label={'Administraci\u00f3n al registrar'}
    />
    <p>Huella de valores <code>{record.values_digest}</code></p>
    <p>Recibo <code>{record.receipt.submission_digest}</code></p>
  </details>
  <div class="action-row">
    {#if canManage && !historical && record.status === 'recorded'}<button
        class="primary"
        disabled={disabled || historyBusy}
        onclick={() => onedit('correct')}>Corregir {singular}</button
      ><button
        class="secondary"
        disabled={disabled || historyBusy}
        onclick={() => onedit('withdraw')}>Retirar {singular}</button
      >{/if}
    <button
      class="text-button"
      disabled={disabled || historyBusy}
      aria-expanded={history}
      onclick={() => (history = !history)}
      >{history ? 'Ocultar' : 'Ver'} historial de {singular}</button
    >
  </div>
  {#if history}<FactHistory
      {api}
      {family}
      id={record.id}
      {ondenied}
      {onselect}
      {disabled}
      bind:busy={historyBusy}
    />{/if}
</section>
