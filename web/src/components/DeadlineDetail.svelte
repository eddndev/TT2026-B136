<script>
  import DeadlineCalculation from './DeadlineCalculation.svelte';
  import DeadlineHistory from './DeadlineHistory.svelte';
  import DeadlineCapturedMaterial from './DeadlineCapturedMaterial.svelte';
  import DeadlineProfileDetails from './DeadlineProfileDetails.svelte';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import { roles } from '../lib/documents.mjs';
  export let api,
    scoped,
    record,
    ondenied,
    onedit,
    onselect,
    canManage = false,
    disabled = false,
    historical = false;
  let history = false,
    historyBusy = false,
    materialOpen = false;
</script>

<section class="card fact-detail deadline-detail" aria-label="Detalle de plazo">
  <div class="section-heading">
    <h2>{record.definition.title}</h2>
    <span class="badge" class:info={record.status === 'active'}
      >{record.status === 'active' ? 'Activo' : 'Retirado'}</span
    >
  </div>
  <p>
    Revisi&#243;n {record.revision} / {historical
      ? 'Consultada exactamente'
      : 'Actual al consultar'}
  </p>
  {#if historical}<p class="notice">
      Esta es la revisi&#243;n seleccionada. Puede existir una posterior.
    </p>
    <button class="secondary" disabled={disabled || historyBusy} onclick={() => onselect(record.id)}
      >Consultar plazo actual</button
    >{/if}
  <p>
    Responsable registrado: <strong>{record.responsible.email}</strong> / {roles[
      record.responsible.role
    ]}
  </p>
  <DeadlineCalculation calculation={record.calculation} />
  <section aria-label="Atencion del plazo">
    <h3>Atenci&#243;n declarada</h3>
    {#if record.attention.status === 'recorded'}<p>{factTimeLabel(record.attention.occurred_at)}</p>
      <p class="case-multiline">{record.attention.statement}</p>
      <p>{record.attention.locator}</p>
    {:else}<p>Pendiente de declaraci&#243;n de atenci&#243;n.</p>{/if}
  </section>
  {#if record.reason}<p class="case-multiline">Motivo: {record.reason}</p>{/if}
  <div class="action-row">
    {#if canManage && !historical && record.status === 'active'}
      <button class="primary" disabled={disabled || historyBusy} onclick={() => onedit('correct')}
        >Corregir plazo</button
      >
      <button
        class="secondary"
        disabled={disabled || historyBusy}
        onclick={() => onedit('set_attention')}>Declarar atenci&#243;n</button
      >
      <button class="secondary" disabled={disabled || historyBusy} onclick={() => onedit('retire')}
        >Retirar plazo</button
      >
    {/if}
    <button
      class="text-button"
      disabled={disabled || historyBusy}
      aria-expanded={history}
      onclick={() => (history = !history)}>{history ? 'Ocultar' : 'Ver'} historial de plazo</button
    >
  </div>
  {#if history}<DeadlineHistory
      api={scoped}
      id={record.id}
      {ondenied}
      {onselect}
      {disabled}
      bind:busy={historyBusy}
    />{/if}
  <details bind:open={materialOpen}>
    <summary>Consultar insumos y fuentes de esta revisi&#243;n</summary>
    {#if materialOpen}<DeadlineCapturedMaterial {api} {record} {ondenied} />{/if}
  </details>
  <DeadlineProfileDetails
    {api}
    caseId={record.case_id}
    captured={record.calculation.profile}
    {ondenied}
  />
  <details>
    <summary>Autor y recibo de esta captura</summary>
    <p><code>{record.id}</code> / Revisi&#243;n {record.revision}</p>
    <p>{record.recorded_by.email} / {deadlineInstantLabel(record.recorded_at)}</p>
    <p>Confirmaci&#243;n: <code>{record.receipt.submission_digest}</code></p>
    <p>Contenido revisado: <code>{record.receipt.review_digest}</code></p>
    <p>Captura: <code>{record.receipt.capture_digest}</code></p>
  </details>
</section>
