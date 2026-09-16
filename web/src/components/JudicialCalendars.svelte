<script>
  import { onMount, onDestroy } from 'svelte';
  import CalendarList from './CalendarList.svelte';
  import CalendarDetail from './CalendarDetail.svelte';
  import CalendarEditor from './CalendarEditor.svelte';
  import { calendarDenied, canCalendars } from '../lib/judicial-calendar-labels.mjs';
  export let api, user;
  const scoped = api.judicialCalendars();
  let filters = { status: 'published', jurisdiction: '', entityCode: '' },
    applied = { ...filters },
    rows = [],
    selected = null,
    historical = false,
    action = null,
    editorKey = 0,
    editorBase = null;
  let cursors = [undefined],
    index = 0,
    next,
    more = false,
    busy = false,
    opening = false,
    editorBusy = false,
    error = '',
    notice = '',
    denied = false,
    alive = true,
    listGeneration = 0,
    detailGeneration = 0;
  $: pending = busy || opening || editorBusy || !!action;
  $: manage = canCalendars(user.role, 'manage') && !denied;
  function fail(failure) {
    error = failure.message;
    if (calendarDenied(failure)) {
      listGeneration++;
      detailGeneration++;
      rows = [];
      selected = null;
      action = null;
      denied = true;
      busy = false;
      opening = false;
    }
  }
  async function load(position = 0) {
    const generation = ++listGeneration;
    busy = true;
    error = '';
    rows = [];
    more = false;
    index = position;
    try {
      const page = await scoped.list({ ...applied, afterId: cursors[position] });
      if (alive && generation === listGeneration) {
        rows = page.calendars;
        more = page.has_more;
        next = page.next_after_id;
        denied = false;
        if (
          selected &&
          rows.some((row) => row.id === selected.id && row.revision > selected.revision)
        )
          historical = true;
      }
    } catch (failure) {
      if (alive && generation === listGeneration) fail(failure);
    } finally {
      if (alive && generation === listGeneration) busy = false;
    }
  }
  async function open(id, revision) {
    if (action) return;
    const generation = ++detailGeneration;
    selected = null;
    opening = true;
    error = '';
    notice = '';
    try {
      const row =
        revision === undefined ? await scoped.get(id) : await scoped.revision(id, revision);
      if (alive && generation === detailGeneration) {
        selected = row;
        historical = revision !== undefined;
      }
    } catch (failure) {
      if (alive && generation === detailGeneration) fail(failure);
    } finally {
      if (alive && generation === detailGeneration) opening = false;
    }
  }
  function edit(next) {
    if (pending || !manage) return;
    action = next;
    editorBase = next === 'publish' ? null : selected;
    editorKey++;
    notice = '';
  }
  async function confirmed(row, exact) {
    if (!alive) return;
    selected = row;
    historical = exact;
    notice = 'Calendario guardado con revisi\u00f3n y recibo.';
    cursors = [undefined];
    await load();
    if (alive) action = null;
  }
  function apply() {
    applied = { ...filters };
    cursors = [undefined];
    load();
  }
  onMount(() => load());
  onDestroy(() => {
    alive = false;
    listGeneration++;
    detailGeneration++;
    scoped.dispose();
  });
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">CONFIGURACI&#211;N DEL DESPACHO</span>
    <h1>Calendarios jurisdiccionales</h1>
    <p>&#193;mbitos, fuentes y clasificaci&#243;n civil con historia exacta.</p>
  </div>
</div>
<p class="notice">
  Estas reglas declaradas no calculan vencimientos ni determinan notificaciones. Su aplicabilidad
  requiere el &#225;mbito y las fuentes expresos.
</p>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
{#if notice}<p class="notice success" role="status">{notice}</p>{/if}
<CalendarList
  {rows}
  bind:filters
  onapply={apply}
  onselect={open}
  {busy}
  disabled={opening || editorBusy || !!action}
  {more}
  {index}
  onprevious={() => load(index - 1)}
  onnext={() => {
    cursors = [...cursors.slice(0, index + 1), next];
    load(index + 1);
  }}
  onrefresh={() => {
    cursors = [undefined];
    load();
  }}
  canManage={manage}
  onnew={() => edit('publish')}
/>
{#if opening}<p role="status">Consultando revisi&#243;n del calendario...</p>{/if}
{#if action}{#key editorKey}<CalendarEditor
      {api}
      {user}
      {action}
      record={editorBase}
      bind:busy={editorBusy}
      onconfirmed={confirmed}
      oncancel={() => (action = null)}
      ondenied={fail}
    />{/key}{/if}
{#if selected && !action}{#key `${selected.id}/${selected.revision}`}<CalendarDetail
      api={scoped}
      record={selected}
      {historical}
      canManage={manage}
      disabled={pending}
      onedit={edit}
      onselect={open}
      oncurrent={() => open(selected.id)}
      ondenied={fail}
    />{/key}{/if}
