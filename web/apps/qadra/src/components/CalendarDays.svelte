<script>
  import { onMount, onDestroy } from 'svelte';
  import { calendarMonth, moveCalendarMonth, civilLabel } from '../lib/judicial-calendar-time.mjs';
  import {
    calendarStates,
    calendarWeekdays,
    calendarDenied,
  } from '../lib/judicial-calendar-labels.mjs';
  import CalendarSources from './CalendarSources.svelte';
  export let api, record, ondenied;
  let month = record.values.coverage.from.slice(0, 7),
    applied = month,
    days = [],
    selected = null,
    mode = 'month',
    busy = false,
    error = '',
    alive = true,
    generation = 0;
  $: range = calendarMonth(applied);
  const symbols = { countable: 'C', excluded: 'E', unresolved: '?', outside_coverage: '--' };
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
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    generation++;
  });
</script>

<section class="calendar-days" aria-label="D&#237;as de esta revisi&#243;n" aria-busy={busy}>
  <div class="section-heading">
    <h3>D&#237;as seg&#250;n esta revisi&#243;n</h3>
    <div class="action-row">
      <button class="secondary" aria-pressed={mode === 'month'} onclick={() => (mode = 'month')}
        >Ver mes</button
      ><button class="secondary" aria-pressed={mode === 'list'} onclick={() => (mode = 'list')}
        >Ver lista de d&#237;as</button
      >
    </div>
  </div>
  <form
    class="calendar-month-controls"
    onsubmit={(event) => {
      event.preventDefault();
      load();
    }}
  >
    <button
      class="secondary"
      type="button"
      disabled={busy || !moveCalendarMonth(applied, -1)}
      onclick={() => load(moveCalendarMonth(applied, -1))}
      aria-label="Mes anterior">Anterior</button
    >
    <label
      >Mes de consulta<input
        type="month"
        min="0001-01"
        max="9999-12"
        bind:value={month}
        disabled={busy}
      /></label
    >
    <button class="secondary" disabled={busy}>Consultar mes</button>
    <button
      class="secondary"
      type="button"
      disabled={busy || !moveCalendarMonth(applied, 1)}
      onclick={() => load(moveCalendarMonth(applied, 1))}
      aria-label="Mes siguiente">Siguiente</button
    >
  </form>
  <p class="hint">
    Clasificaci&#243;n civil declarada; no calcula vencimientos, guardias ni notificaciones.
  </p>
  <div class="calendar-legend">
    {#each Object.entries(calendarStates) as [state, label]}<span data-state={state}
        >{symbols[state]} {label}</span
      >{/each}
  </div>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button class="secondary" disabled={busy} onclick={() => load()}
      >Consultar d&#237;as de nuevo</button
    >{/if}
  {#if busy}<p role="status">Consultando d&#237;as exactos...</p>{/if}
  {#if days.length}<div
      class:calendar-month={mode === 'month'}
      class:calendar-day-list={mode === 'list'}
      aria-label="Fechas clasificadas"
    >
      {#if mode === 'month'}{#each calendarWeekdays as name}<span
            class="calendar-weekday"
            aria-hidden="true">{name.slice(0, 2)}</span
          >{/each}{#each Array(range.firstWeekday - 1) as _}<span aria-hidden="true"
          ></span>{/each}{/if}
      {#each days as day (day.date)}<button
          class="calendar-day"
          data-state={day.state}
          class:chosen={selected?.date === day.date}
          aria-pressed={selected?.date === day.date}
          aria-label={`${civilLabel(day.date)}: ${calendarStates[day.state]}`}
          onclick={() => (selected = day)}
        >
          <span>{mode === 'month' ? Number(day.date.slice(-2)) : civilLabel(day.date)}</span><small
            >{mode === 'month' ? symbols[day.state] : calendarStates[day.state]}</small
          >
        </button>{/each}
    </div>{/if}
  {#if selected}<section class="case-comparison" aria-label="Detalle del d&#237;a">
      <h4>{civilLabel(selected.date)} / {calendarStates[selected.state]}</h4>
      {#if selected.state === 'outside_coverage'}<p>
          Esta revisi&#243;n no cubre la fecha; no se aplica una regla por defecto.
        </p>
      {:else}<p>
          {selected.origin === 'exception'
            ? 'Excepci\u00f3n declarada'
            : 'Patr\u00f3n semanal declarado'}
        </p>
        <p class="case-multiline">{selected.explanation}</p>
        <CalendarSources
          sources={record.values.sources.filter((s) => selected.source_ids.includes(s.id))}
        />{/if}
    </section>{/if}
</section>
