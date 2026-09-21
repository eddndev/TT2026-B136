<script>
  import { onMount, onDestroy } from 'svelte';
  import { resourceActKinds } from '../lib/procedural-resource-values.mjs';
  import { resourceDenied } from '../lib/procedural-resource-errors.mjs';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  export let api,
    caseId,
    resource,
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false;
  const scoped = api.caseResources(caseId);
  let rows = [],
    revision = '',
    candidate = null,
    more = false,
    cursor;
  let alive = true,
    generation = 0,
    error = '';
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
        if (resourceDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function history(beforeRevision) {
    const value = await work(() => scoped.history(resource.id, { limit: 20, beforeRevision }));
    if (!value) return;
    const acts = value.revisions.filter((row) => row.act !== null);
    rows = beforeRevision ? [...rows, ...acts] : acts;
    more = value.has_more;
    cursor = value.next_before_revision;
  }
  async function read(event) {
    revision = event.currentTarget.value;
    candidate = null;
    if (!revision) return;
    const value = await work(() => scoped.revision(resource.id, Number(revision)));
    if (value?.act) candidate = value;
  }
  onMount(() => history());
  onDestroy(() => {
    alive = false;
    generation++;
    scoped.dispose();
    busy = false;
  });
</script>

<section class="case-comparison" aria-label="Seleccionar acto del recurso" aria-busy={busy}>
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button type="button" class="secondary" disabled={disabled || busy} onclick={() => history()}
      >Consultar actos de nuevo</button
    >{/if}
  <label
    >Acto del recurso
    <select value={revision} onchange={read} disabled={disabled || busy}>
      <option value="">Selecciona una captura exacta del acto</option>
      {#each rows as row}<option value={String(row.revision)}>
          {resourceActKinds[row.act.values.kind]} / Acto revisi&#243;n {row.act.revision} / Recurso revisi&#243;n
          {row.revision}
        </option>{/each}
    </select>
  </label>
  {#if !busy && !error && rows.length === 0}
    <p class="case-muted">
      {more
        ? 'No hay actos en esta p\u00e1gina del historial.'
        : 'No hay actos registrados en el historial consultado.'}
    </p>
  {/if}
  {#if more}<button
      type="button"
      class="secondary"
      disabled={disabled || busy}
      onclick={() => history(cursor)}>Cargar actos anteriores</button
    >{/if}
  {#if candidate}
    <p>
      {resourceActKinds[candidate.act.values.kind]} / {factTimeLabel(
        candidate.act.values.occurred_at,
      )}
    </p>
    <p class="case-multiline">{candidate.act.values.statement}</p>
    <p>Acto revisi&#243;n {candidate.act.revision} / Recurso revisi&#243;n {candidate.revision}</p>
    <button
      type="button"
      class="primary"
      disabled={disabled || busy}
      onclick={() => onselected(candidate)}>Usar este acto</button
    >
  {/if}
  <button type="button" class="text-button" disabled={disabled || busy} onclick={oncancel}
    >Cerrar selector de acto</button
  >
</section>
