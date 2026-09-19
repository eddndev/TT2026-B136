<script>
  import { onMount, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import AlertCard from './AlertCard.svelte';
  import AlertPreferences from './AlertPreferences.svelte';
  import { basicCase } from '../lib/case-administration.mjs';
  import { alertFailure, alertTimeLabel } from '../lib/alerts-presentation.mjs';
  import { alertKey, compareAlertKeys } from '../lib/alerts-primitives.mjs';
  export let api,
    user,
    onopen,
    filters = null;
  const scoped = api.alerts(user.id);
  let read = filters?.read ?? 'all',
    state = filters?.state ?? 'active';
  let applied = { read, state },
    rows = [],
    checked = null,
    more = false,
    next = null;
  let busy = false,
    opening = false,
    preferencesOpen = false,
    error = '',
    notice = '';
  let alive = true,
    denied = false,
    loaded = false,
    generation = 0,
    openGeneration = 0;
  function deny(failure, row) {
    generation++;
    openGeneration++;
    busy = false;
    opening = false;
    error = alertFailure(failure);
    if (row) rows = rows.filter((value) => value.subject.case_id !== row.subject.case_id);
    else {
      rows = [];
      denied = true;
      preferencesOpen = false;
    }
  }
  async function load(append = false) {
    if (denied || (append && (!more || busy))) return;
    const request = ++generation;
    openGeneration++;
    opening = false;
    busy = true;
    error = '';
    notice = '';
    if (!append) {
      rows = [];
      more = false;
      next = null;
      loaded = false;
      checked = null;
    }
    try {
      const result = await scoped.list({
        ...applied,
        limit: 20,
        ...(append ? { cursor: next } : {}),
      });
      if (!alive || request !== generation) return;
      const collection = new Map((append ? rows : []).map((row) => [row.id, row]));
      for (const row of result.alerts) collection.set(row.id, row);
      rows = [...collection.values()].sort((a, b) => compareAlertKeys(alertKey(b), alertKey(a)));
      checked = result.checked_at;
      more = result.has_more;
      next = result.next_cursor;
      loaded = true;
    } catch (failure) {
      if (alive && request === generation) {
        error = alertFailure(failure);
        if ([403, 404].includes(failure.status)) deny(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function apply(event) {
    event?.preventDefault();
    applied = { read, state };
    filters = { ...applied };
    load();
  }
  function recorded(row, at) {
    if (!alive) return;
    generation++;
    busy = false;
    rows = rows
      .map((value) => (value.id === row.id ? row : value))
      .filter((value) => applied.read !== 'unread' || value.read_at === null);
    checked = at;
    notice = 'Estado de lectura actualizado.';
  }
  async function open(row) {
    if (busy || opening || denied) return;
    const request = ++openGeneration;
    opening = true;
    error = '';
    let administration;
    try {
      const detail = await scoped.get(row.id);
      if (!alive || request !== openGeneration) return;
      const captured = detail.alert;
      administration = api.caseAdministration(captured.subject.case_id);
      const record = await administration.get();
      if (!alive || request !== openGeneration) return;
      if (record.id !== captured.subject.case_id)
        throw new Error('El expediente no corresponde a esta alerta.');
      const subject = captured.subject;
      onopen(basicCase(record), {
        kind: subject.kind,
        case_id: subject.case_id,
        revision: captured.origin.revision,
        ...(subject.kind === 'hearing' ? { hearing_id: subject.id } : { deadline_id: subject.id }),
      });
    } catch (failure) {
      if (alive && request === openGeneration) {
        error = alertFailure(failure);
        if ([403, 404].includes(failure.status)) deny(failure, row);
      }
    } finally {
      administration?.dispose();
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

<section class="alerts-workspace" aria-label="Mis alertas">
  <div class="page-heading">
    <div>
      <span class="eyebrow">TU SEGUIMIENTO PERSONAL</span>
      <h1>Mis alertas</h1>
      <p>Avisos de tus audiencias y plazos, con su origen y estado de lectura.</p>
    </div>
    <button
      class="secondary"
      disabled={denied || preferencesOpen}
      onclick={() => {
        preferencesOpen = true;
        notice = '';
      }}
    >
      Preferencias de alertas
    </button>
  </div>
  <p class="notice">
    La fecha capturada no acredita la vigencia actual. Abre la revisi&#243;n de origen para
    consultar el expediente y contrastar su estado.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if notice}<p class="notice success" role="status">{notice}</p>{/if}
  {#if preferencesOpen}<AlertPreferences
      api={scoped}
      ondenied={deny}
      oncancel={() => {
        preferencesOpen = false;
      }}
      onconfirmed={() => {
        preferencesOpen = false;
        notice = 'Preferencias guardadas.';
      }}
    />{/if}
  {#if !denied}
    <section class="card alerts-query" aria-label="Consulta de alertas">
      <form class="alerts-filters" onsubmit={apply}>
        <label
          >Lectura<select bind:value={read}
            ><option value="all">Todas</option><option value="unread">Sin leer</option></select
          ></label
        >
        <label
          >Estado de alerta<select bind:value={state}
            ><option value="active">Activas</option><option value="all">Activas y resueltas</option
            ></select
          ></label
        >
        <button class="primary" type="submit">Consultar alertas</button>
        <button class="secondary" type="button" disabled={busy || opening} onclick={() => load()}
          ><Icon name="clock" size={16} />Actualizar alertas</button
        >
      </form>
      {#if checked}<p class="hint">Consulta comprobada: {alertTimeLabel(checked)}</p>{/if}
    </section>
    {#if busy}<p role="status">Consultando alertas...</p>{/if}
    {#if opening}<p role="status">Consultando el expediente y la revisi&#243;n de origen...</p>{/if}
    <div class="alerts-list" aria-busy={busy || opening}>
      {#each rows as row (row.id)}<AlertCard
          {row}
          api={scoped}
          disabled={busy || opening}
          onopen={open}
          onread={recorded}
          ondenied={deny}
        />{/each}
    </div>
    {#if loaded && !busy && !rows.length && !more && !error}<div class="card empty-state">
        <Icon name="checklist" size={30} />
        <p>No hay alertas en esta consulta.</p>
      </div>{/if}
    {#if more}<div class="alerts-continuation">
        <p class="hint">Consulta parcial: faltan alertas por consultar.</p>
        <button class="secondary" disabled={busy || opening} onclick={() => load(true)}
          >Cargar m&#225;s alertas</button
        >
      </div>{/if}
  {/if}
</section>
