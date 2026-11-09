<script>
  // The counts row at the bottom of the overview heading, the moment they were
  // read and the action that reads them again. While a read is pending or
  // after it fails no figure is shown, so a stale or zero count never stands
  // in for the current one. Urgent counts above zero carry dashboard-urgent.
  import Button from './Button.svelte';
  export let value = null;
  export let busy = false;
  export let error = '';
  export let onrefresh;
  const figure = 'text-figure tracking-tight tabular-nums';
  // Each count spans three rows of the list (label, figure, detail) so the
  // figures of a row line up when a label wraps.
  const cell = 'row-span-3 grid grid-rows-subgrid gap-y-1 bg-surface px-6 py-5';
  const timestamp = (instant) =>
    new Intl.DateTimeFormat('es-MX', { dateStyle: 'medium', timeStyle: 'short' }).format(
      new Date(instant),
    );
  $: overdue = value?.deadlines_overdue > 0;
  $: soon = value?.deadlines_due_48h > 0;
</script>

<div class="border-t border-line">
  {#if busy}
    <p class="px-6 py-5 text-body text-muted" role="status">
      Consultando los indicadores actuales...
    </p>
  {:else if error}
    <div class="px-6 py-5"><p class="notice error" role="alert">{error}</p></div>
  {:else if value}
    <dl class="grid grid-cols-2 gap-px border-b border-line bg-line md:grid-cols-5">
      <div class={cell} data-metric="active_cases">
        <dt class="text-body text-muted">Expedientes activos</dt>
        <dd><strong class={`${figure} text-ink`}>{value.active_cases}</strong></dd>
      </div>
      <div class={cell} class:dashboard-urgent={overdue} data-metric="deadlines_overdue">
        <dt class="text-body text-muted">Plazos vencidos</dt>
        <dd>
          <strong class={`${figure} ${overdue ? 'text-danger' : 'text-ink'}`}
            >{value.deadlines_overdue}</strong
          >
        </dd>
      </div>
      <div class={cell}>
        <dt class="text-body text-muted">Vencen en 7 d&#237;as</dt>
        <dd data-metric="deadlines_due_7d">
          <strong class={`${figure} text-ink`}>{value.deadlines_due_7d}</strong>
        </dd>
        <dd
          class="text-caption text-muted"
          class:dashboard-urgent={soon}
          data-metric="deadlines_due_48h"
        >
          <strong class="font-medium text-ink tabular-nums">{value.deadlines_due_48h}</strong> en menos
          de 48 horas
        </dd>
      </div>
      <div class={cell} data-metric="deadlines_unresolved">
        <dt class="text-body text-muted">Por revisar</dt>
        <dd>
          <strong class={`${figure} text-ink`}>{value.deadlines_unresolved}</strong>
        </dd>
      </div>
      <div class={`${cell} col-span-2 md:col-span-1`} data-metric="pending_contracts">
        <dt class="text-body text-muted">Contratos en firma</dt>
        <dd>
          <strong class={`${figure} text-ink`}>{value.pending_contracts}</strong>
        </dd>
      </div>
    </dl>
  {/if}
  <div class="flex flex-wrap items-center gap-x-3 gap-y-1 px-6 pt-4 text-body text-muted">
    {#if value && !busy && !error}
      <p>Consulta del <time datetime={value.checked_at}>{timestamp(value.checked_at)}</time></p>
    {/if}
    <Button
      variant="text"
      type="button"
      aria-label="Actualizar indicadores"
      disabled={busy}
      onclick={onrefresh}>Actualizar</Button
    >
  </div>
  <p class="px-6 pt-1 pb-4 text-caption text-muted">
    Los plazos incluyen expedientes cerrados administrativamente. Se excluyen los retirados y
    atendidos; los que requieren revisi&#243;n no se presentan como fechas vigentes.
  </p>
</div>
