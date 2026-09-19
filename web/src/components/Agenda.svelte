<script>
  import { onMount, onDestroy } from 'svelte';
  import AgendaFilters from './AgendaFilters.svelte';
  import AgendaList from './AgendaList.svelte';
  import AgendaCalendar from './AgendaCalendar.svelte';
  import { basicCase } from '../lib/case-administration.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import {
    initialAgendaSelection,
    agendaSelection,
    agendaRecord,
    agendaIntent,
    mergeAgendaItems,
  } from './agenda-presentation.mjs';
  import '../styles/agenda.css';

  export let api,
    onopen,
    oncalendars = () => {},
    filters = null;
  const scoped = api.agenda();
  const initial = agendaSelection(initialAgendaSelection(filters));
  let chosen = initial.filters,
    applied = initial.query,
    range = initial.range;
  let rows = [],
    next = null,
    complete = false,
    checkedAt = null;
  let error = '',
    busy = false,
    opening = false;
  let alive = true,
    generation = 0,
    openGeneration = 0;

  async function load(append = false) {
    if (append && (busy || complete || next === null)) return;
    const request = ++generation;
    const cursor = append ? next : undefined;
    openGeneration++;
    opening = false;
    busy = true;
    error = '';
    if (!append) {
      rows = [];
      next = null;
      complete = false;
      checkedAt = null;
    }
    try {
      const result = await scoped.list({ ...applied, ...(cursor === undefined ? {} : { cursor }) });
      if (!alive || request !== generation) return;
      rows = mergeAgendaItems(append ? rows : [], result.items);
      next = result.next_cursor;
      complete = result.complete;
      checkedAt = result.checked_at;
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) {
          rows = [];
          next = null;
          complete = false;
          checkedAt = null;
        }
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }

  function apply(value) {
    generation++;
    openGeneration++;
    busy = false;
    opening = false;
    try {
      const selection = agendaSelection(value);
      chosen = selection.filters;
      filters = { ...chosen };
      applied = selection.query;
      range = selection.range;
      load();
    } catch (failure) {
      error = failure.message;
    }
  }

  async function open(item) {
    const record = agendaRecord(item);
    const request = ++openGeneration,
      list = generation;
    const administration = api.caseAdministration(record.case_id);
    opening = true;
    error = '';
    try {
      const detail = await administration.get();
      if (!alive || request !== openGeneration || list !== generation) return;
      if (detail.id !== record.case_id)
        throw new Error('El expediente no corresponde a la actividad seleccionada.');
      onopen(basicCase(detail), agendaIntent(item));
    } catch (failure) {
      if (alive && request === openGeneration && list === generation) {
        error = failure.message;
        if ([403, 404].includes(failure.status))
          rows = rows.filter((value) => agendaRecord(value).case_id !== record.case_id);
      }
    } finally {
      administration.dispose();
      if (alive && request === openGeneration) opening = false;
    }
  }

  onMount(() => {
    filters = { ...chosen };
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
    openGeneration++;
    scoped.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">PROGRAMACI&#211;N DEL DESPACHO</span>
    <h1>Agenda del despacho</h1>
    <p>Audiencias y vencimientos operativos de tus expedientes autorizados.</p>
  </div>
  <button class="secondary" onclick={oncalendars}>Calendarios jurisdiccionales</button>
</div>
<section
  class="card hearing-agenda combined-agenda"
  aria-label="Agenda combinada"
  aria-busy={busy || opening}
>
  <AgendaFilters value={chosen} onapply={apply} />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <div class="section-heading">
    <div>
      <h2>Actividades en el periodo</h2>
      <p class="hint">{range.from} hasta {range.until} (excluido) / UTC{chosen.offset}</p>
    </div>
    <button class="secondary" disabled={busy || opening} onclick={() => load()}
      >Actualizar Agenda</button
    >
  </div>
  {#if checkedAt}<p class="hint">&#218;ltima consulta: {deadlineInstantLabel(checkedAt)}</p>{/if}
  {#if busy}<p role="status">Consultando Agenda...</p>{/if}
  {#if !complete && !error && checkedAt}
    <p class="notice" role="status">Consulta parcial: faltan actividades por consultar.</p>
  {/if}
  {#if chosen.view === 'week' || chosen.view === 'month'}
    <AgendaCalendar
      {rows}
      {range}
      view={chosen.view}
      offset={chosen.offset}
      {complete}
      onselect={open}
      disabled={busy || opening}
    />
  {:else}
    <AgendaList
      {rows}
      {range}
      view={chosen.view}
      offset={chosen.offset}
      {complete}
      onselect={open}
      disabled={busy || opening}
    />
  {/if}
  {#if !busy && complete && !rows.length && !error}
    <p>No hay actividades en esta consulta.</p>
  {/if}
  <div class="pagination">
    <span class="hint">{rows.length} actividades cargadas</span>
    {#if !complete && next !== null}
      <button class="secondary" disabled={busy || opening} onclick={() => load(true)}
        >Cargar m&#225;s actividades</button
      >
    {/if}
  </div>
  <p class="hint">
    Las actividades pueden cambiar entre consultas. Actualiza la agenda antes de actuar. Los plazos
    que requieren revisi&#243;n se consultan dentro de su expediente.
  </p>
</section>
