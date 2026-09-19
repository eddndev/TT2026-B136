<script>
  import { onMount, onDestroy } from 'svelte';
  import {
    hearingKinds,
    hearingStatus,
    hearingTimeLabel,
    hearingDenied,
  } from '../lib/hearings.mjs';
  export let api,
    caseId,
    kind,
    onselected,
    ondenied,
    disabled = false,
    busy = false;
  const scoped = kind === 'hearing' ? api.caseHearings(caseId) : api.deadlines(caseId);
  let roots = [],
    revisions = [],
    rootId = '',
    revision = '',
    candidate = null;
  let rootMore = false,
    historyMore = false,
    rootCursor,
    historyCursor;
  let alive = true,
    generation = 0,
    error = '';
  let mounted = false,
    initialRequested = false;
  $: if (mounted && !disabled && !busy && !initialRequested) {
    initialRequested = true;
    list();
  }
  function title(row) {
    return kind === 'hearing'
      ? `${hearingKinds[row.kind || row.values?.kind]?.label || 'Audiencia'} / ${hearingTimeLabel(row.scheduled_at || row.values?.scheduled_at)}`
      : row.definition?.title || row.title;
  }
  function clear() {
    candidate = null;
    onselected(null);
  }
  async function work(task) {
    if (busy || disabled) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const value = await task();
      if (alive && request === generation) return value;
    } catch (failure) {
      if (alive && request === generation) {
        error = failure.message;
        if (failure.status === 401 || hearingDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function list(afterId) {
    const value = await work(() => scoped.list({ status: 'all', limit: 20, afterId }));
    if (!value) return;
    roots = afterId
      ? [...roots, ...value[kind === 'hearing' ? 'hearings' : 'deadlines']]
      : value[kind === 'hearing' ? 'hearings' : 'deadlines'];
    rootMore = value.has_more;
    rootCursor = value.next_after_id;
  }
  async function history(beforeRevision) {
    const value = await work(() => scoped.history(rootId, { limit: 20, beforeRevision }));
    if (!value) return;
    revisions = beforeRevision ? [...revisions, ...value.revisions] : value.revisions;
    historyMore = value.has_more;
    historyCursor = value.next_before_revision;
  }
  async function changeRoot(event) {
    clear();
    rootId = event.currentTarget.value;
    revision = '';
    revisions = [];
    historyMore = false;
    if (rootId) await history();
  }
  async function changeRevision(event) {
    clear();
    revision = event.currentTarget.value;
    if (!revision) return;
    const value = await work(() => scoped.revision(rootId, Number(revision)));
    if (value) candidate = value;
  }
  onMount(() => {
    mounted = true;
  });
  onDestroy(() => {
    mounted = false;
    alive = false;
    generation++;
    scoped.dispose();
    busy = false;
  });
</script>

<section class="case-comparison" aria-label="Seleccionar actividad existente" aria-busy={busy}>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button type="button" class="secondary" disabled={busy || disabled} onclick={() => list()}
      >Consultar actividades de nuevo</button
    >{/if}
  <label
    >Actividad existente
    <select value={rootId} onchange={changeRoot} disabled={disabled || busy}>
      <option value="">Selecciona una actividad</option>
      {#each roots as row}<option value={row.id}>{title(row)}</option>{/each}
    </select>
  </label>
  {#if !busy && !error && roots.length === 0}<p class="case-muted">
      No hay actividades disponibles de este tipo.
    </p>{/if}
  {#if rootMore}<button
      type="button"
      class="secondary"
      disabled={disabled || busy}
      onclick={() => list(rootCursor)}>Cargar m&#225;s actividades</button
    >{/if}
  {#if rootId}
    <label
      >Revisi&#243;n de la actividad
      <select value={revision} onchange={changeRevision} disabled={disabled || busy}>
        <option value="">Selecciona una revisi&#243;n exacta</option>
        {#each revisions as row}<option value={String(row.revision)}>
            Revisi&#243;n {row.revision} / {kind === 'hearing'
              ? hearingStatus[row.status]
              : row.status === 'retired'
                ? 'Retirado'
                : 'Activo'}
          </option>{/each}
      </select>
    </label>
    {#if historyMore}<button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => history(historyCursor)}>Cargar revisiones anteriores de la actividad</button
      >{/if}
  {/if}
  {#if candidate}
    <p>{title(candidate)} / Revisi&#243;n {candidate.revision}</p>
    {#if kind === 'hearing'}<p class="case-multiline">{candidate.values.venue}</p>{/if}
    <p class="case-muted">
      Esta captura hist&#243;rica conserva la revisi&#243;n elegida. La actividad actual se consulta
      por separado.
    </p>
    <button
      type="button"
      class="primary"
      disabled={disabled || busy}
      onclick={() => onselected(candidate)}>Usar esta revisi&#243;n</button
    >
  {/if}
</section>
