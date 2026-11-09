<script>
  // The coming agenda of the overview: the next activities of the combined
  // Agenda as a short list beside one month whose busy days carry the navy
  // dot. Selecting a row or a day opens the Agenda. The parent remounts it for
  // each account; refresh() queries again.
  import { getContext, onMount, onDestroy } from 'svelte';
  import Button from './Button.svelte';
  import MonthCalendar from './MonthCalendar.svelte';
  import UpcomingList from './UpcomingList.svelte';
  import { civilToday, monthOf } from '../lib/month-grid.mjs';
  import {
    comingDays,
    localOffset,
    dashboardAgendaSelection,
    dashboardAgendaRows,
    dashboardAgendaDay,
    dashboardAgendaLoader,
  } from './dashboard-agenda.mjs';
  export let api, onnavigate;
  let className = '';
  export { className as class };
  const session = getContext('session-drafts');
  const loader = dashboardAgendaLoader({ scoped: api.agenda(), session });
  const today = civilToday(),
    offset = localOffset();
  const { range, query } = dashboardAgendaSelection(today, offset);
  const first = monthOf(range.from),
    last = monthOf(range.until);
  let month = first,
    items = [],
    complete = true,
    busy = true,
    error = '';
  export async function refresh() {
    busy = true;
    error = '';
    const outcome = await loader.load(query);
    if (!outcome) return;
    busy = false;
    items = outcome.page?.items ?? [];
    complete = outcome.page?.complete ?? true;
    error = outcome.error ?? '';
  }
  $: rows = dashboardAgendaRows(items, offset);
  $: describe = dashboardAgendaDay(items, offset);
  onMount(refresh);
  onDestroy(() => loader.stop());
</script>

<section
  class={`@container surface-card p-6 ${className}`}
  aria-labelledby="dashboard-agenda-title"
  aria-busy={busy}
>
  <div class="flex items-baseline justify-between gap-4">
    <h2 id="dashboard-agenda-title" class="text-card-title text-ink">
      Pr&#243;ximos plazos y audiencias
    </h2>
    <Button
      variant="text"
      type="button"
      class="shrink-0 whitespace-nowrap"
      onclick={() => onnavigate('agenda')}>Ver agenda</Button
    >
  </div>
  <p class="mt-1 text-body text-muted">Los pr&#243;ximos {comingDays} d&#237;as</p>
  {#if error}<p class="notice error mt-4" role="alert">{error}</p>{/if}
  <div class="mt-6 grid gap-8 @2xl:grid-cols-[minmax(0,1fr)_16rem]">
    <div class="min-w-0">
      <UpcomingList
        title="Actividades pr&#243;ximas"
        level={3}
        headingClass="sr-only"
        {rows}
        empty={busy || error ? null : 'Sin plazos ni audiencias en los pr\u00f3ximos d\u00edas'}
        onselect={() => onnavigate('agenda')}
      />
      {#if busy}<p class="py-4 text-body text-muted" role="status">Consultando la agenda...</p>{/if}
      {#if !busy && !error && (!complete || items.length > rows.length)}
        <p class="text-body text-muted">
          Hay m&#225;s actividades en este periodo; cons&#250;ltalas en la agenda
        </p>
      {/if}
    </div>
    <MonthCalendar
      label="Calendario de pr&#243;ximas actividades"
      months={1}
      {month}
      min={first}
      max={last}
      {today}
      {describe}
      onselect={() => onnavigate('agenda')}
      onmonth={(value) => (month = value)}
    />
  </div>
</section>
