<script>
  import Button from './Button.svelte';
  import { getContext, onMount, onDestroy } from 'svelte';
  import AgendaFilters from './AgendaFilters.svelte';
  import MonthCalendar from './MonthCalendar.svelte';
  import UpcomingList from './UpcomingList.svelte';
  import ResourceHearingDetail from './ResourceHearingDetail.svelte';
  import PrecautionaryHearingDetail from './PrecautionaryHearingDetail.svelte';
  import { basicCase } from '../lib/case-administration.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import { agendaCivilDay } from '../lib/agenda-periods.mjs';
  import { monthOf } from '../lib/month-grid.mjs';
  import { agendaCalendarDay, agendaRow } from './agenda-rows.mjs';
  import {
    initialAgendaSelection,
    agendaSelection,
    agendaRecord,
    agendaIntent,
    agendaGroups,
    mergeAgendaItems,
  } from './agenda-presentation.mjs';

  export let api,
    onopen,
    oncalendars = () => {},
    filters = null;
  const scoped = api.agenda();
  const session = getContext('session-drafts');
  const initial = agendaSelection(initialAgendaSelection(filters));
  let chosen = initial.filters,
    applied = initial.query,
    range = initial.range;
  // First month shown by the calendar, the only one below md. It opens on
  // the reference date of each applied view (a custom range on its first
  // day), so a week that starts in the previous month still shows the chosen
  // day; the month controls move it without querying again.
  const anchorMonth = (filters, period) =>
    monthOf(filters.view === 'custom' ? period.from : filters.date);
  let visibleMonth = anchorMonth(chosen, range);
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
  let ownDetail = null,
    ownKind = null,
    ownCase = null,
    ownClient = null,
    detailTrigger = null;

  const principal = () => session?.principal();
  const admitted = () => alive && (!session || session.canAdmit());
  const currentOpen = (request, list, actor) => {
    const current = principal();
    return (
      admitted() &&
      request === openGeneration &&
      list === generation &&
      current?.id === actor?.id &&
      current?.email === actor?.email &&
      current?.role === actor?.role
    );
  };
  function invalidateDetail() {
    openGeneration++;
    ownClient?.dispose();
    ownClient = null;
    ownDetail = null;
    ownKind = null;
    ownCase = null;
    opening = false;
  }
  function closeDetail() {
    invalidateDetail();
    detailTrigger?.focus();
    detailTrigger = null;
  }

  async function load(append = false) {
    if (append && (busy || complete || next === null)) return;
    const request = ++generation;
    const cursor = append ? next : undefined;
    invalidateDetail();
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
    invalidateDetail();
    busy = false;
    opening = false;
    try {
      const selection = agendaSelection(value);
      chosen = selection.filters;
      filters = { ...chosen };
      applied = selection.query;
      range = selection.range;
      visibleMonth = anchorMonth(chosen, range);
      load();
    } catch (failure) {
      error = failure.message;
    }
  }

  async function open(item) {
    if (!admitted()) return;
    invalidateDetail();
    detailTrigger = document.activeElement;
    const record = agendaRecord(item);
    const request = ++openGeneration,
      list = generation,
      actor = principal() ? { ...principal() } : null;
    const administration = api.caseAdministration(record.case_id);
    opening = true;
    error = '';
    try {
      const detail = await administration.get();
      if (!currentOpen(request, list, actor)) return;
      if (detail.id !== record.case_id)
        throw new Error('El expediente no corresponde a la actividad seleccionada.');
      if (['resource_hearing', 'precautionary_hearing'].includes(item.kind)) {
        const client =
          item.kind === 'precautionary_hearing'
            ? api.casePrecautionaryHearings(record.case_id)
            : api.caseResourceHearings(record.case_id, record.resource_id);
        ownClient = client;
        const creation = await client.exact(record);
        if (!currentOpen(request, list, actor)) return;
        ownCase = basicCase(detail);
        ownKind = item.kind;
        ownDetail = creation;
      } else {
        onopen(basicCase(detail), agendaIntent(item));
      }
    } catch (failure) {
      if (currentOpen(request, list, actor)) {
        error = failure.message;
        ownDetail = null;
        ownCase = null;
        if ([403, 404].includes(failure.status))
          rows = rows.filter((value) => agendaRecord(value).case_id !== record.case_id);
      }
    } finally {
      administration.dispose();
      if (alive && request === openGeneration) opening = false;
    }
  }

  // A clicked day becomes the reference date of the applied view (a custom
  // range becomes that day), keeping the applied kind, status and offset.
  function chooseDay(date) {
    apply({ ...chosen, view: chosen.view === 'custom' ? 'day' : chosen.view, date });
  }
  function civilNow(offset) {
    try {
      return agendaCivilDay(new Date().toISOString(), offset);
    } catch {
      return null;
    }
  }
  $: groups = agendaGroups(rows, chosen.offset);
  $: describeDay = agendaCalendarDay({ groups, range, complete });
  $: listed = rows.map((item) => agendaRow(item, chosen.offset));
  $: today = civilNow(chosen.offset);
  $: empty = error
    ? null
    : complete
      ? busy
        ? null
        : 'No hay actividades en esta consulta.'
      : checkedAt
        ? 'A\u00fan no hay actividades cargadas.'
        : null;

  onMount(() => {
    filters = { ...chosen };
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
    invalidateDetail();
    scoped.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">PROGRAMACI&#211;N DEL DESPACHO</span>
    <h1>Agenda del despacho</h1>
    <p>Audiencias y vencimientos operativos de tus expedientes autorizados.</p>
  </div>
  <Button variant="secondary" onclick={oncalendars}>Calendarios jurisdiccionales</Button>
</div>
<section
  class="card hearing-agenda min-w-0"
  aria-label="Agenda combinada"
  aria-busy={busy || opening}
>
  <AgendaFilters value={chosen} onapply={apply} />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <MonthCalendar
    label="Calendario de la agenda"
    month={visibleMonth}
    {today}
    selected={chosen.view === 'custom' ? null : chosen.date}
    describe={describeDay}
    onselect={chooseDay}
    onmonth={(month) => (visibleMonth = month)}
  />
  <div class="mt-8">
    <UpcomingList
      title="Actividades en el periodo"
      rows={listed}
      {empty}
      onselect={open}
      disabled={busy || opening}
    >
      <div slot="header" class="mt-1 grid gap-2">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <p class="hint">{range.from} hasta {range.until} (excluido) / UTC{chosen.offset}</p>
          <Button variant="secondary" disabled={busy} onclick={() => load()}
            >Actualizar Agenda</Button
          >
        </div>
        {#if checkedAt}<p class="hint">
            &#218;ltima consulta: {deadlineInstantLabel(checkedAt)}
          </p>{/if}
        {#if busy}<p role="status">Consultando Agenda...</p>{/if}
        {#if !complete && !error && checkedAt}
          <p class="notice" role="status">Consulta parcial: faltan actividades por consultar.</p>
        {/if}
      </div>
    </UpcomingList>
  </div>
  <div class="pagination">
    <span class="hint">{rows.length} actividades cargadas</span>
    {#if !complete && next !== null}
      <Button variant="secondary" disabled={busy || opening} onclick={() => load(true)}
        >Cargar m&#225;s actividades</Button
      >
    {/if}
  </div>
  <p class="hint">
    Las actividades pueden cambiar entre consultas. Actualiza la agenda antes de actuar. Los plazos
    que requieren revisi&#243;n se consultan dentro de su expediente.
  </p>
</section>
{#if ownDetail}
  {#if ownKind === 'precautionary_hearing'}
    <PrecautionaryHearingDetail value={ownDetail} caseRecord={ownCase} onclose={closeDetail} />
  {:else}
    <ResourceHearingDetail value={ownDetail} caseRecord={ownCase} onclose={closeDetail} />
  {/if}
{/if}
