<script>
  import { onMount, onDestroy } from 'svelte';
  import AgendaFilters from './AgendaFilters.svelte';
  import AgendaList from './AgendaList.svelte';
  import { defaultAgendaFilters, agendaQuery } from '../lib/hearing-agenda.mjs';
  import { basicCase } from '../lib/case-administration.mjs';
  export let api,
    onopen,
    filters = null;
  const scoped = api.hearingAgenda();
  let initial = filters || defaultAgendaFilters(),
    applied = agendaQuery(initial),
    offset = initial.offset;
  let rows = [],
    cursors = [undefined],
    index = 0,
    next,
    more = false,
    error = '',
    busy = false,
    opening = false;
  let alive = true,
    generation = 0,
    openGeneration = 0;
  async function load(position = 0) {
    const request = ++generation;
    openGeneration++;
    opening = false;
    busy = true;
    error = '';
    rows = [];
    more = false;
    index = position;
    try {
      const result = await scoped.list({ ...applied, after: cursors[position] });
      if (!alive || request !== generation) return;
      rows = result.hearings;
      more = result.has_more;
      next = result.next_after;
    } catch (failure) {
      if (alive && request === generation) {
        rows = [];
        error = failure.message;
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function apply(value) {
    try {
      const query = agendaQuery(value);
      filters = { ...value };
      applied = query;
      offset = value.offset;
      cursors = [undefined];
      load();
    } catch (failure) {
      error = failure.message;
    }
  }
  async function open(row) {
    const request = ++openGeneration,
      list = generation,
      administration = api.caseAdministration(row.case_id);
    opening = true;
    error = '';
    try {
      const detail = await administration.get();
      if (alive && request === openGeneration && list === generation) {
        if (detail.id !== row.case_id)
          throw new Error('El expediente no corresponde a la audiencia seleccionada.');
        onopen(basicCase(detail), {
          case_id: row.case_id,
          hearing_id: row.id,
          revision: row.revision,
        });
      }
    } catch (failure) {
      if (alive && request === openGeneration && list === generation) {
        error = failure.message;
        if ([403, 404].includes(failure.status))
          rows = rows.filter((item) => item.case_id !== row.case_id);
      }
    } finally {
      administration.dispose();
      if (alive && request === openGeneration) opening = false;
    }
  }
  onMount(() => load());
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
    <h1>Agenda de audiencias</h1>
    <p>Citas de los expedientes autorizados para tu cuenta.</p>
  </div>
</div>
<section class="card hearing-agenda" aria-label="Agenda de audiencias" aria-busy={busy || opening}>
  <AgendaFilters value={initial} onapply={apply} />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <div class="section-heading">
    <h2>Audiencias en el rango</h2>
    <button
      class="secondary"
      disabled={busy || opening}
      onclick={() => {
        cursors = [undefined];
        load();
      }}>Actualizar Agenda</button
    >
  </div>
  <p class="hint">
    Agrupadas por d&#237;a en UTC{offset}. Cada cita muestra tambi&#233;n su fecha, hora y desfase
    originales.
  </p>
  {#if busy}<p role="status">Consultando Agenda...</p>{/if}
  <AgendaList {rows} {offset} onselect={open} disabled={busy || opening} />
  {#if !busy && !rows.length && !error}<p>No hay audiencias en esta consulta.</p>{/if}
  <div class="pagination">
    <span class="hint">{rows.length} citas en esta p&#225;gina</span>
    <div class="action-row">
      <button class="secondary" disabled={busy || opening || !index} onclick={() => load(index - 1)}
        >Anterior</button
      >
      <button
        class="secondary"
        disabled={busy || opening || !more}
        onclick={() => {
          cursors = [...cursors.slice(0, index + 1), next];
          load(index + 1);
        }}>Siguiente</button
      >
    </div>
  </div>
  <p class="hint">
    La programaci&#243;n puede cambiar entre consultas. Esta vista no registra resultados ni calcula
    plazos o avisos.
  </p>
</section>
