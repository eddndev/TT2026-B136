<script>
  import HearingSupport from './HearingSupport.svelte';
  import { resultSource } from '../lib/hearing-result-errors.mjs';
  export let draft,
    caseId,
    documents,
    ondenied,
    disabled = false,
    pending = false,
    draftContext = null,
    ondiscard = () => {};
  $: support = draft.provenance.support
    ? { ...draft.provenance.support, id: draft.provenance.support.document_id }
    : null;
  function select(value) {
    draft.provenance.support = value
      ? {
          document_id: value.id,
          version: value.version,
          digest: value.digest,
          name: value.name,
          uploaded: value.uploaded,
        }
      : null;
  }
</script>

<fieldset class="case-offenses" disabled={disabled || pending}>
  <legend>Procedencia del relato</legend>
  <label
    >Procedencia<select bind:value={draft.provenance.kind}>
      <option value="">Selecciona una fuente</option>
      {#each Object.entries(resultSource) as [value, label]}<option {value}>{label}</option>{/each}
    </select></label
  >
  <label
    >Localizador de la fuente{draft.provenance.kind === 'operator_note' ? ' (opcional)' : ''}<input
      bind:value={draft.provenance.reference}
    /></label
  >
  <p class="hint">
    Describe la referencia oral, escrita o de trabajo. Los localizadores se conservan como texto.
  </p>
</fieldset>
<details class="hearing-result-optional">
  <summary>Soporte documental (opcional)</summary>
  {#if draft.provenance.support}<details>
      <summary>Identidad exacta del soporte</summary><code
        >{draft.provenance.support.document_id}</code
      >
    </details>{/if}
  <HearingSupport
    api={documents}
    {caseId}
    {ondenied}
    {disabled}
    {draftContext}
    label="Soporte del resultado"
    chooseLabel="Elegir soporte del resultado"
    uploadLabel="Cargar soporte del resultado"
    value={support}
    bind:pending
    optional
    onselected={select}
    ondiscard={() => {
      select(null);
      ondiscard();
    }}
  />
</details>
