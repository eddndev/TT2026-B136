<script>
  import CalendarValues from './CalendarValues.svelte';
  import CalendarDays from './CalendarDays.svelte';
  import CalendarHistory from './CalendarHistory.svelte';
  import { calendarStatus } from '../lib/judicial-calendar-labels.mjs';
  export let record,
    api,
    onedit,
    onselect,
    oncurrent,
    ondenied,
    canManage = false,
    historical = false,
    disabled = false;
  let history = false;
</script>

<section class="card calendar-detail" aria-label="Detalle del calendario">
  <div class="section-heading">
    <h2>{record.values.scope.title}</h2>
    <span class="badge" class:info={record.status === 'published'}
      >{calendarStatus[record.status]}</span
    >
  </div>
  <p class="hint">
    Revisi&#243;n {record.revision}{historical
      ? ' hist\u00f3rica consultada exactamente'
      : ' actual al consultar'}
  </p>
  {#if historical}<p class="notice">
      Revisi&#243;n hist&#243;rica. Su clasificaci&#243;n se conserva aunque exista una
      publicaci&#243;n posterior o el calendario se haya retirado.
    </p>
    <button class="secondary" {disabled} onclick={oncurrent}>Consultar calendario actual</button
    >{/if}
  {#if record.status === 'retired'}<p class="notice">
      El retiro conserva la historia y es terminal. No declara la derogaci&#243;n de una norma.
    </p>{/if}
  {#if record.reason}<p class="case-multiline">Motivo: {record.reason}</p>{/if}
  <CalendarDays {api} {record} {ondenied} />
  <CalendarValues values={record.values} />
  <details>
    <summary>Identidad, recibo y autor de esta revisi&#243;n</summary>
    <p>Calendario: <code>{record.id}</code></p>
    <p>Valores: <code>{record.values_digest}</code></p>
    <p>Operaci&#243;n: <code>{record.receipt.operation_id}</code></p>
    <p>Recibo: <code>{record.receipt.submission_digest}</code></p>
    <p>
      {record.recorded_by.email} / <time datetime={record.recorded_at}>{record.recorded_at}</time>
    </p>
    <p class="hint">
      La huella identifica los metadatos declarados, no el contenido remoto de las referencias.
    </p>
  </details>
  <div class="action-row">
    {#if canManage && record.status === 'published' && !historical}<button
        class="primary"
        {disabled}
        onclick={() => onedit('replace')}>Reemplazar calendario</button
      ><button class="secondary" {disabled} onclick={() => onedit('retire')}
        >Retirar calendario</button
      >{/if}<button class="text-button" {disabled} onclick={() => (history = !history)}
      >{history ? 'Ocultar historial del calendario' : 'Ver historial del calendario'}</button
    >
  </div>
  {#if history}<CalendarHistory {api} id={record.id} {onselect} {ondenied} {disabled} />{/if}
</section>
