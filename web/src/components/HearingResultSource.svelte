<script>
  import StagePicker from './StagePicker.svelte';
  import StageSupportSummary from './StageSupportSummary.svelte';
  import UploadDocument from './UploadDocument.svelte';
  import { resultSource } from '../lib/hearing-result-errors.mjs';
  export let draft,
    caseId,
    documents,
    ondenied,
    disabled = false,
    pending = false;
  let picking = false,
    pickerBusy = false,
    uploadBusy = false,
    uploader;
  $: pending = pickerBusy || uploadBusy;
  function select(row) {
    draft.provenance.support = {
      document_id: row.id,
      version: row.version,
      digest: row.digest,
      name: row.name,
      uploaded: row.uploaded,
    };
    picking = false;
  }
</script>

<fieldset class="case-offenses" disabled={disabled || pending}>
  <legend>Procedencia del relato</legend>
  <label
    >Procedencia<select bind:value={draft.provenance.kind}
      ><option value="">Selecciona una fuente</option
      >{#each Object.entries(resultSource) as [value, label]}<option {value}>{label}</option
        >{/each}</select
    ></label
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
  {#if draft.provenance.support}<StageSupportSummary record={draft.provenance.support} />
    <details>
      <summary>Identidad exacta del soporte</summary><code
        >{draft.provenance.support.document_id}</code
      >
    </details>
    {#if draft.provenance.support.uploaded}<p class="notice" role="status">
        El soporte se guard&#243;. Falta confirmar el registro.
      </p>{/if}
    <button
      type="button"
      class="text-button"
      disabled={disabled || pending}
      onclick={() => (draft.provenance.support = null)}>Quitar soporte del borrador</button
    >{/if}
  <div class="action-row">
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => (picking = true)}>Elegir soporte del resultado</button
    ><button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => uploader.open()}>Cargar soporte del resultado</button
    >
  </div>
  {#if picking}<StagePicker
      api={documents}
      {caseId}
      {ondenied}
      {disabled}
      bind:busy={pickerBusy}
      onselected={select}
      oncancel={() => (picking = false)}
    />{/if}
</details>
<UploadDocument
  api={documents}
  {ondenied}
  {disabled}
  bind:this={uploader}
  bind:busy={uploadBusy}
  onuploaded={(row) => select({ ...row, uploaded: true })}
/>
