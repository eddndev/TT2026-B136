<script>
  import { onDestroy } from 'svelte';
  import FactResolutionPicker from './FactResolutionPicker.svelte';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import { factDeclarationLabel, classLabels } from './fact-field-labels.mjs';
  export let api,
    caseId,
    value = null,
    onselected = () => {},
    ondenied,
    disabled = false,
    busy = false,
    draft = null,
    canApply = () => true;
  const scoped = api.caseResolutions(caseId);
  let choosing = false,
    roots = [],
    selectedId = null,
    selected = null,
    pending = false,
    pickerBusy = false,
    more = false,
    cursor,
    error = '',
    alive = true;
  let picker,
    pickerDraft = draft;
  export function captureDraft() {
    return {
      selectedId: selectedId ?? pickerDraft?.selectedId ?? null,
      revision: picker?.captureDraft()?.revision ?? pickerDraft?.revision ?? null,
    };
  }
  $: busy = pending || pickerBusy;
  async function list(afterId) {
    if (pending || pickerBusy || disabled || !canApply()) return;
    choosing = true;
    pending = true;
    error = '';
    selectedId = null;
    try {
      const page = await scoped.list({ limit: 20, status: 'all', afterId });
      if (!alive || !canApply()) return;
      roots = page.resolutions;
      more = page.has_more;
      cursor = page.next_after_id;
    } catch (failure) {
      if (!alive || !canApply()) return;
      error = failure.message;
      if ([401, 403].includes(failure.status) || failure.code === 'case_not_found') {
        roots = [];
        selected = null;
        value = null;
        choosing = false;
        ondenied(failure);
      }
    } finally {
      if (alive) pending = false;
    }
  }
  function select(row) {
    if (!canApply()) return;
    const changed = value?.id !== row.id || value?.revision !== row.revision;
    value = { id: row.id, revision: row.revision };
    selected = row;
    choosing = false;
    selectedId = null;
    pickerDraft = null;
    if (changed) onselected(row);
  }
  onDestroy(() => {
    alive = false;
    scoped.dispose();
    busy = false;
  });
</script>

<section class="case-comparison" aria-label="Resoluci&#243;n impugnada" aria-busy={busy}>
  <h4>Resoluci&#243;n impugnada</h4>
  {#if value}
    <p>
      Revisi&#243;n exacta {value.revision}{selected
        ? ` / ${selected.status === 'withdrawn' ? 'Retirada' : 'Registrada'}`
        : ''}
    </p>
    {#if selected}<p class="case-multiline">{selected.values.summary}</p>{/if}
    <details><summary>Identidad de la resoluci&#243;n</summary><code>{value.id}</code></details>
  {:else}<p class="hint">
      Selecciona una resoluci&#243;n y consulta la revisi&#243;n que deseas vincular.
    </p>{/if}
  <button type="button" class="secondary" disabled={disabled || busy} onclick={() => list()}>
    Elegir resoluci&#243;n hist&#243;rica
  </button>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if choosing}
    {#if !selectedId}
      {#each roots as row}
        <div class="hearing-result-picker-row">
          <p>{factDeclarationLabel(row.class, classLabels)} / {factTimeLabel(row.issued_at)}</p>
          <p>
            Revisi&#243;n {row.revision} / {row.status === 'withdrawn' ? 'Retirada' : 'Registrada'}
          </p>
          <button
            type="button"
            class="secondary"
            disabled={disabled || busy}
            aria-label={`Consultar revisiones de resoluci\u00f3n ${row.id}`}
            onclick={() => {
              if (canApply()) selectedId = row.id;
            }}>Consultar revisiones de la resoluci&#243;n</button
          >
        </div>
      {/each}
      {#if !roots.length && !pending && !error}<p>No hay resoluciones en esta consulta.</p>{/if}
      {#if more}<button
          type="button"
          class="secondary"
          disabled={disabled || busy}
          onclick={() => list(cursor)}>Siguientes resoluciones</button
        >{/if}
      <button type="button" class="text-button" disabled={busy} onclick={() => (choosing = false)}
        >Cerrar lista de resoluciones</button
      >
    {:else}
      <FactResolutionPicker
        {api}
        {caseId}
        resolutionId={selectedId}
        {ondenied}
        {disabled}
        {canApply}
        draft={pickerDraft?.selectedId === selectedId ? pickerDraft : null}
        bind:this={picker}
        bind:busy={pickerBusy}
        onselected={select}
        oncancel={() => {
          pickerDraft = captureDraft();
          selectedId = null;
        }}
      />
    {/if}
  {/if}
  <p class="hint">
    La selecci&#243;n conserva su historia; no declara interposici&#243;n ni modifica la etapa.
  </p>
</section>
