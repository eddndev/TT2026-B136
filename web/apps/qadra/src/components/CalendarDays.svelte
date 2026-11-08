<script>
  import Button from './Button.svelte';
  import MonthCalendar from './MonthCalendar.svelte';
  import UpcomingList from './UpcomingList.svelte';
  import CalendarSources from './CalendarSources.svelte';
  import { onMount, onDestroy } from 'svelte';
  import { calendarMonth, civilLabel } from '../lib/judicial-calendar-time.mjs';
  import { calendarStates, calendarDenied } from '../lib/judicial-calendar-labels.mjs';
  import { civilToday, dayTone } from '../lib/month-grid.mjs';
  import {
    calendarCoverageMonths,
    calendarDayDescriber,
    calendarDayOrigin,
    calendarDayRow,
    calendarLegend,
  } from '../lib/judicial-calendar-days.mjs';
  // The classified days of one revision, one month per query: a month grid
  // or a dated list, the legend and the detail of the selected day. The
  // month controls stay within the coverage; "Mes de consulta" reaches any
  // civil month.
  export let api, record, ondenied;
  let month = record.values.coverage.from.slice(0, 7),
    applied = month,
    days = [],
    selected = null,
    mode = 'month',
    today = null,
    busy = false,
    error = '',
    alive = true,
    generation = 0;
  $: bounds = calendarCoverageMonths(record.values.coverage);
  $: describe = calendarDayDescriber(days);
  async function load(value = month) {
    const request = ++generation;
    busy = true;
    error = '';
    selected = null;
    days = [];
    try {
      const range = calendarMonth(value);
      const row = await api.days(record.id, record.revision, range, record.values_digest);
      if (alive && request === generation) {
        days = row.days;
        applied = value;
        month = value;
      }
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if (calendarDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  // A date without a loaded row is ignored (its day button is disabled).
  function choose(date) {
    selected = days.find((day) => day.date === date) ?? selected;
  }
  onMount(() => {
    today = civilToday();
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
  });
</script>

<section class="calendar-days" aria-label="D&#237;as de esta revisi&#243;n" aria-busy={busy}>
  <div class="section-heading">
    <h3>D&#237;as seg&#250;n esta revisi&#243;n</h3>
    <div class="action-row">
      <Button variant="secondary" aria-pressed={mode === 'month'} onclick={() => (mode = 'month')}
        >Ver mes</Button
      ><Button variant="secondary" aria-pressed={mode === 'list'} onclick={() => (mode = 'list')}
        >Ver lista de d&#237;as</Button
      >
    </div>
  </div>
  <form
    class="flex flex-wrap items-end gap-2.5"
    onsubmit={(event) => {
      event.preventDefault();
      load();
    }}
  >
    <label class="grid min-w-0 flex-1 gap-1.5 text-label text-ink sm:max-w-56"
      >Mes de consulta<input
        type="month"
        min="0001-01"
        max="9999-12"
        class="w-full min-w-0 rounded-md border border-line-strong bg-surface px-3 py-2 text-body text-ink disabled:opacity-60"
        bind:value={month}
        disabled={busy}
      /></label
    >
    <Button variant="secondary" disabled={busy}>Consultar mes</Button>
  </form>
  <p class="hint">
    Clasificaci&#243;n civil declarada; no calcula vencimientos, guardias ni notificaciones.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <Button variant="secondary" disabled={busy} onclick={() => load()}
      >Consultar d&#237;as de nuevo</Button
    >{/if}
  {#if busy}<p role="status">Consultando d&#237;as exactos...</p>{/if}
  <div class="grid gap-8 md:grid-cols-2 md:gap-x-14">
    <div class="min-w-0">
      {#if mode === 'month'}
        <MonthCalendar
          months={1}
          label="Fechas clasificadas"
          month={applied}
          min={bounds.min}
          max={bounds.max}
          {today}
          selected={selected?.date ?? null}
          {describe}
          onselect={choose}
          onmonth={(target) => load(target)}
          disabled={busy}
          headingLevel={4}
        />
      {:else}
        <UpcomingList
          title="Fechas clasificadas"
          level={4}
          rows={days.map((day) => calendarDayRow(day, selected?.date ?? null))}
          onselect={(day) => (selected = day)}
          disabled={busy}
        />
      {/if}
    </div>
    <div class="grid min-w-0 content-start gap-4">
      <ul aria-label="Leyenda de estados" class="flex flex-wrap gap-2 text-caption">
        {#each calendarLegend as entry (entry.state)}
          <li class={`rounded-full px-2.5 py-1 ring-1 ring-line ${dayTone(entry.tone)}`}>
            <span aria-hidden="true" class="font-medium">{entry.symbol}</span>
            {entry.label}
          </li>
        {/each}
      </ul>
      {#if selected}<section class="case-comparison" aria-label="Detalle del d&#237;a">
          <h4 class="font-medium text-ink">
            {civilLabel(selected.date)} / {calendarStates[selected.state]}
          </h4>
          <p>{calendarDayOrigin(selected)}</p>
          {#if selected.state !== 'outside_coverage'}
            <p class="case-multiline">{selected.explanation}</p>
            <CalendarSources
              sources={record.values.sources.filter((s) => selected.source_ids.includes(s.id))}
            />{/if}
        </section>
      {:else if days.length}<p class="text-body text-muted">
          Selecciona un d&#237;a para consultar su regla declarada.
        </p>{/if}
    </div>
  </div>
</section>
