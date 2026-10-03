<script>
  import { onDestroy } from 'svelte';
  import ResourceActivityActPicker from './ResourceActivityActPicker.svelte';
  import { resourceActKinds } from '../lib/procedural-resource-values.mjs';
  export let api,
    caseId,
    resource,
    selection,
    ondenied,
    disabled = false,
    busy = false,
    savedInputs = null;
  let choosing = savedInputs?.choosing ?? false,
    selected = null;
  let picker;
  export function captureInputs() {
    return { choosing, picker: picker?.captureInputs() ?? savedInputs?.picker ?? null };
  }
  function select(row) {
    selection = {
      ...selection,
      act: {
        id: row.act.id,
        revision: row.act.revision,
        resource_revision: row.revision,
        capture_digest: row.receipt.capture_digest,
      },
    };
    selected = row;
    choosing = false;
    savedInputs = null;
  }
  function clear() {
    selection = { ...selection, act: null };
    selected = null;
    choosing = false;
    savedInputs = null;
  }
  onDestroy(() => {
    busy = false;
  });
</script>

<p>Captura del recurso: {resource.values.title} / Revisi&#243;n {selection.resource.revision}</p>
<p class="hint">
  El plazo conserva el perfil, las fuentes y el calendario declarados. Vincular el recurso o su acto
  no sustituye esas entradas del c&#243;mputo.
</p>
<section aria-label="Acto opcional del recurso">
  <h4>Acto del recurso (opcional)</h4>
  {#if selection.act}
    <p>
      {selected ? resourceActKinds[selected.act.values.kind] : 'Acto seleccionado'} / Revisi&#243;n {selection
        .act.revision} / Recurso revisi&#243;n {selection.act.resource_revision}
    </p>
    {#if selected}<p class="case-multiline">{selected.act.values.statement}</p>{/if}
    <button type="button" class="text-button" disabled={disabled || busy} onclick={clear}
      >Quitar acto</button
    >
  {:else}<p class="case-muted">Sin acto seleccionado.</p>{/if}
  {#if choosing}
    <ResourceActivityActPicker
      bind:this={picker}
      savedInputs={savedInputs?.picker ?? null}
      {api}
      {caseId}
      {resource}
      {ondenied}
      {disabled}
      bind:busy
      onselected={select}
      oncancel={() => (choosing = false)}
    />
  {:else}
    <button
      type="button"
      class="secondary"
      disabled={disabled || busy}
      onclick={() => (choosing = true)}>Elegir acto del recurso</button
    >
  {/if}
</section>
