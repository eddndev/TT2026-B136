<script>
  import { onDestroy, setContext } from 'svelte';
  import { writable } from 'svelte/store';
  import CaseStages from './CaseStages.svelte';
  import CaseHearings from './CaseHearings.svelte';
  import CaseContext from './CaseContext.svelte';
  import CaseAdministration from './CaseAdministration.svelte';
  import Documents from './Documents.svelte';
  import Participants from './Participants.svelte';
  import { staffCase, basicCase } from '../lib/case-administration.mjs';
  export let api, user, record, view, onnavigate, onchange, onupdate;
  export let hearingIntent = null,
    onhearingintent = () => {};
  export let intent = null,
    onintent = () => {};
  const staff = staffCase(user.role);
  const scoped = staff ? api.caseAdministration(record.id) : null;
  const state = writable({
    closed: record.administration?.administrative_status === 'closed',
    refresh,
  });
  setContext('case-administration', state);
  let current = record,
    alive = true,
    generation = 0,
    busy = false,
    error = '',
    denied = false;
  function update(result) {
    if (
      !alive ||
      denied ||
      !current ||
      result.id !== current.id ||
      result.administration.revision < current.administration.revision
    )
      return;
    current = basicCase(result);
    state.set({ closed: result.administration.administrative_status === 'closed', refresh });
    onupdate(current);
  }
  function deny(failure) {
    generation++;
    denied = true;
    error = failure.message;
    busy = false;
    current = null;
  }
  async function refresh() {
    if (!staff || denied) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const result = await scoped.get();
      if (alive && request === generation) update(result);
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) deny(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  const unwatch = api.watchCase(record.id, () => {
    if (!alive || denied) return;
    state.set({ closed: true, refresh });
    refresh();
  });
  onDestroy(() => {
    alive = false;
    generation++;
    unwatch();
    scoped?.dispose();
  });
</script>

{#if denied}<section class="card empty-state">
    <h1>Expediente no disponible</h1>
    <p class="notice error" role="alert">{error}</p>
    <button class="secondary" onclick={onchange}>Volver a expedientes</button>
  </section>
{:else}
  <CaseContext record={current} {user} {view} {onnavigate} {onchange} />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if staff && $state.closed}<p class="notice" role="status">
      Expediente cerrado administrativamente. Puedes consultar, verificar y exportar; las
      modificaciones est&#225;n suspendidas.
    </p>{/if}
  {#if busy}<p class="hint" role="status">Consultando estado del expediente...</p>{/if}
  {#if view === 'case-summary'}
    {#if staff}<CaseAdministration
        {api}
        {scoped}
        {user}
        record={current}
        onupdate={update}
        ondenied={deny}
        onrefresh={refresh}
        {onnavigate}
      />
    {:else}<div class="page-heading"><h1>Resumen del expediente</h1></div>
      <section class="card">
        <h2>{current.title}</h2>
        <p>Referencia interna: {current.reference}</p>
        <p class="hint">Tu cuenta permite consultar los datos b&#225;sicos de este expediente.</p>
      </section>{/if}
  {:else if view === 'stages' && staff}<CaseStages
      {api}
      {user}
      record={current}
      {onnavigate}
      loading={busy}
      ondenied={deny}
    />
  {:else if view === 'hearings' && staff}<CaseHearings
      {api}
      {user}
      record={current}
      {onnavigate}
      ondenied={deny}
      intent={hearingIntent}
      onintent={onhearingintent}
    />
  {:else if view === 'participants'}<Participants {api} {user} caseRecord={current} />
  {:else}<Documents {api} {user} caseRecord={current} {intent} {onintent} />{/if}
{/if}
