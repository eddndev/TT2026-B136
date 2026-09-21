<script>
  import { onDestroy } from 'svelte';
  import ResourceActivityTargetPicker from './ResourceActivityTargetPicker.svelte';
  import ResourceActivityActPicker from './ResourceActivityActPicker.svelte';
  import { resourceActKinds } from '../lib/procedural-resource-values.mjs';
  export let api,
    caseId,
    resource,
    selection,
    ondenied,
    disabled = false,
    busy = false;
  let kind = selection.target?.kind || '',
    targetBusy = false,
    actBusy = false;
  let choosingAct = false,
    selectedAct = null;
  $: busy = targetBusy || actBusy;
  function changeKind(event) {
    kind = event.currentTarget.value;
    selection = { ...selection, target: null };
  }
  function selectTarget(row) {
    const target =
      row === null
        ? null
        : {
            kind,
            id: row.id,
            revision: row.revision,
            ...(kind === 'hearing'
              ? { submission_digest: row.receipt.submission_digest }
              : { capture_digest: row.receipt.capture_digest }),
          };
    selection = { ...selection, target };
  }
  function selectAct(row) {
    selection = {
      ...selection,
      act: {
        id: row.act.id,
        revision: row.act.revision,
        resource_revision: row.revision,
        capture_digest: row.receipt.capture_digest,
      },
    };
    selectedAct = row;
    choosingAct = false;
  }
  function clearAct() {
    selection = { ...selection, act: null };
    selectedAct = null;
    choosingAct = false;
  }
  onDestroy(() => (busy = false));
</script>

<p>Captura del recurso: {resource.values.title} / Revisi&#243;n {selection.resource.revision}</p>
<p class="case-muted">
  Vincular organiza actividades existentes. No cambia su fecha, su c&#243;mputo ni la etapa del
  expediente.
</p>
<label
  >Tipo de actividad
  <select value={kind} onchange={changeKind} disabled={disabled || busy}>
    <option value="">Selecciona el tipo de actividad</option>
    <option value="hearing">Audiencia</option>
    <option value="deadline">Plazo</option>
  </select>
</label>
{#if kind}
  {#key `${caseId}:${kind}`}
    <ResourceActivityTargetPicker
      {api}
      {caseId}
      {kind}
      {ondenied}
      onselected={selectTarget}
      disabled={disabled || actBusy}
      bind:busy={targetBusy}
    />
  {/key}
{/if}
{#if selection.target}<p class="case-muted">
    Actividad elegida: revisi&#243;n {selection.target.revision}.
  </p>{/if}
<section aria-label="Acto opcional del recurso">
  <h4>Acto del recurso (opcional)</h4>
  {#if selection.act}
    <p>
      {selectedAct ? resourceActKinds[selectedAct.act.values.kind] : 'Acto seleccionado'} / Revisi&#243;n
      {selection.act.revision} / Recurso revisi&#243;n {selection.act.resource_revision}
    </p>
    {#if selectedAct}<p class="case-multiline">{selectedAct.act.values.statement}</p>{/if}
    <button type="button" class="text-button" disabled={disabled || busy} onclick={clearAct}
      >Quitar acto</button
    >
  {:else}<p class="case-muted">Sin acto seleccionado.</p>{/if}
  {#if choosingAct}
    {#key `${caseId}:${resource.id}`}
      <ResourceActivityActPicker
        {api}
        {caseId}
        {resource}
        {ondenied}
        onselected={selectAct}
        oncancel={() => (choosingAct = false)}
        disabled={disabled || targetBusy}
        bind:busy={actBusy}
      />
    {/key}
  {:else}
    <button
      type="button"
      class="secondary"
      disabled={disabled || busy}
      onclick={() => (choosingAct = true)}>Elegir acto del recurso</button
    >
  {/if}
</section>
