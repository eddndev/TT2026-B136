<script>
  import { onMount, onDestroy } from 'svelte';
  import CalendarValues from './CalendarValues.svelte';
  import CalendarSources from './CalendarSources.svelte';
  import { deadlineDenied, deadlineFailure } from '../lib/deadline-errors.mjs';
  export let api,
    caseId,
    family = 'profile',
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false;
  let scope = 'case',
    scoped = family === 'profile' ? api.deadlineProfiles(caseId) : api.judicialCalendars();
  let alive = true,
    rows = [],
    revisions = [],
    exact = null,
    selectedId = null,
    more = false,
    historyMore = false,
    cursor,
    historyCursor,
    error = '';
  $: noun = family === 'profile' ? 'perfil' : 'calendario';
  async function work(task) {
    if (busy || disabled) return;
    busy = true;
    error = '';
    try {
      const row = await task();
      if (alive) return row;
    } catch (failure) {
      if (alive) {
        error = deadlineFailure(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function list(afterId) {
    exact = null;
    revisions = [];
    selectedId = null;
    historyMore = false;
    const page = await work(() => scoped.list({ status: 'published', limit: 20, afterId }));
    if (page) {
      rows = family === 'profile' ? page.profiles : page.calendars;
      more = page.has_more;
      cursor = page.next_after_id;
    }
  }
  async function changeScope(next) {
    if (busy || disabled || next === scope) return;
    scoped.dispose();
    scope = next;
    rows = [];
    more = false;
    scoped = api.deadlineProfiles(next === 'global' ? null : caseId);
    await list();
  }
  async function history(id, beforeRevision) {
    exact = null;
    const page = await work(() => scoped.history(id, { limit: 10, beforeRevision }));
    if (page) {
      selectedId = id;
      revisions = page.revisions;
      historyMore = page.has_more;
      historyCursor = page.next_before_revision;
    }
  }
  async function read(revision) {
    exact = null;
    const row = await work(() => scoped.revision(selectedId, revision));
    if (row) exact = row;
  }
  onMount(() => list());
  onDestroy(() => {
    alive = false;
    busy = false;
    scoped.dispose();
  });
</script>

<section class="case-comparison" aria-label={`Elegir ${noun} exacto`} aria-busy={busy}>
  <h4>Elegir {noun} y revisi&#243;n</h4>
  <p class="hint">
    Selecciona una revisi&#243;n publicada; el servidor vuelve a comprobar su disponibilidad al
    confirmar.
  </p>
  {#if family === 'profile'}<label
      >Cat&#225;logo de perfiles<select
        value={scope}
        disabled={disabled || busy}
        onchange={(event) => changeScope(event.currentTarget.value)}
      >
        <option value="case">Disponibles en el expediente (privados y globales)</option>
        <option value="global">Solo globales</option>
      </select></label
    >{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#each rows as row}<div class="hearing-result-picker-row">
      <p>{row.title || row.scope?.title} / Revisi&#243;n {row.revision}</p>
      {#if family === 'profile'}<p>
          {row.scope.kind === 'case' ? 'Privado del expediente' : 'Global'}
        </p>{/if}
      <button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => history(row.id)}>Revisiones de {row.title || row.scope?.title}</button
      >
    </div>{/each}
  {#if !rows.length && !busy}<p>
      No hay {family === 'profile' ? 'perfiles' : 'calendarios'} publicados en esta p&#225;gina.
    </p>{/if}
  <div class="action-row">
    <button type="button" class="secondary" disabled={disabled || busy} onclick={() => list()}
      >Primera p&#225;gina de {noun}</button
    >
    {#if more}<button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => list(cursor)}
        >Siguientes {family === 'profile' ? 'perfiles' : 'calendarios'}</button
      >{/if}
  </div>
  {#if selectedId}<h5>Revisiones del {noun}</h5>
    {#each revisions as row}<button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => read(row.revision)}
        >Consultar {noun} revisi&#243;n {row.revision} ({row.status === 'retired'
          ? 'retirado'
          : 'publicado'})</button
      >{/each}
    {#if historyMore}<button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => history(selectedId, historyCursor)}>Revisiones anteriores del {noun}</button
      >{/if}
  {/if}
  {#if exact}<section class="case-comparison" aria-label={`Revision exacta del ${noun}`}>
      <h5>
        {family === 'profile' ? exact.definition.title : exact.values.scope.title} / Revisi&#243;n {exact.revision}
      </h5>
      {#if family === 'profile'}
        <p class="case-multiline">{exact.definition.description}</p>
        <h5>Condiciones requeridas</h5>
        {#each exact.definition.conditions as condition}<p class="case-multiline">
            {condition.statement}
          </p>{/each}
        <CalendarSources sources={exact.definition.references} />
      {:else}<CalendarValues values={exact.values} />{/if}
      {#if exact.status === 'retired'}<p class="notice">
          Esta revisi&#243;n est&#225; retirada y no puede elegirse para una captura nueva.
        </p>
      {:else}<button
          type="button"
          class="primary"
          disabled={disabled || busy}
          onclick={() => onselected(exact)}>Usar este {noun} exacto</button
        >{/if}
    </section>{/if}
  <button type="button" class="text-button" disabled={busy} onclick={oncancel}
    >Cerrar selector de {noun}</button
  >
</section>
