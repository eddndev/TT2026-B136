<script>
  import { onMount, onDestroy } from 'svelte';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import { factDeclarationLabel, classLabels } from './fact-field-labels.mjs';
  export let api,
    caseId,
    resolutionId,
    onselected,
    oncancel,
    ondenied,
    disabled = false,
    busy = false;
  const scoped = api.caseResolutions(caseId);
  let revisions = [],
    exact = null,
    more = false,
    cursor,
    alive = true,
    error = '';
  async function work(task) {
    if (busy || disabled) return;
    busy = true;
    error = '';
    try {
      const value = await task();
      if (alive) return value;
    } catch (failure) {
      if (alive) {
        error = failure.message;
        if ([401, 403].includes(failure.status) || failure.code === 'case_not_found')
          ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function history(beforeRevision) {
    exact = null;
    const page = await work(() => scoped.history(resolutionId, { limit: 10, beforeRevision }));
    if (page) {
      revisions = page.revisions;
      more = page.has_more;
      cursor = page.next_before_revision;
      exact = null;
    }
  }
  async function read(revision) {
    exact = null;
    const value = await work(() => scoped.revision(resolutionId, revision));
    if (value) exact = value;
  }
  onMount(() => history());
  onDestroy(() => {
    alive = false;
    scoped.dispose();
    busy = false;
  });
</script>

<section
  class="case-comparison"
  aria-label="Elegir revisi&#243;n de la resoluci&#243;n"
  aria-busy={busy}
>
  <h4>Revisiones de la resoluci&#243;n vinculada</h4>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#each revisions as row}<div class="hearing-result-picker-row">
      <p>Revision {row.revision} / {row.status === 'withdrawn' ? 'Retirada' : 'Registrada'}</p>
      <button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={() => read(row.revision)}
        >Consultar resoluci&#243;n revisi&#243;n {row.revision}</button
      >
    </div>{/each}
  {#if more}<button
      type="button"
      class="secondary"
      disabled={disabled || busy}
      onclick={() => history(cursor)}>Revisiones anteriores de la resoluci&#243;n</button
    >{/if}
  {#if exact}<p>
      Revisi&#243;n exacta {exact.revision} / {exact.status === 'withdrawn'
        ? 'Retirada'
        : 'Registrada'}
    </p>
    <p>
      {factDeclarationLabel(exact.values.class, classLabels)} / {factTimeLabel(
        exact.values.issued_at,
      )}
    </p>
    <p class="case-multiline">{exact.values.summary}</p>
    <button
      type="button"
      class="primary"
      disabled={disabled || busy}
      onclick={() => onselected(exact)}>Vincular esta revisi&#243;n de resoluci&#243;n</button
    >
  {/if}
  <button type="button" class="text-button" disabled={busy} onclick={oncancel}
    >Cerrar selector de resoluci&#243;n</button
  >
</section>
