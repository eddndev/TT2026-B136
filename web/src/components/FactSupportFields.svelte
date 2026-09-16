<script>
  import { onDestroy } from 'svelte';
  import StagePicker from './StagePicker.svelte';
  import StageSupportSummary from './StageSupportSummary.svelte';
  import UploadDocument from './UploadDocument.svelte';
  export let value = null,
    api,
    caseId,
    label,
    ondenied,
    disabled = false,
    pending = false;
  const documents = api.caseDocuments(caseId);
  let choosing = false,
    pickerBusy = false,
    uploadBusy = false,
    uploader,
    selected = null,
    uploaded = false;
  $: pending = pickerBusy || uploadBusy;
  function select(row, created = false) {
    selected = row;
    uploaded = created;
    value = {
      document_id: row.id,
      version: row.version,
      digest: row.digest,
      locator: value?.locator || '',
    };
    choosing = false;
  }
  onDestroy(() => documents.dispose());
</script>

<section class="fact-support-fields" aria-label={`Soporte: ${label}`}>
  <h4>Soporte documental de {label} (opcional)</h4>
  {#if value}
    <StageSupportSummary record={{ ...value, name: selected?.name }} />
    <label
      >Localizador documental: {label}<input
        bind:value={value.locator}
        disabled={disabled || pending}
      /></label
    >
    <details><summary>Identidad del soporte</summary><code>{value.document_id}</code></details>
    {#if uploaded}<p class="notice" role="status">
        El documento se guard&#243;. Falta confirmar este registro.
      </p>{/if}
    <button
      type="button"
      class="text-button"
      disabled={disabled || pending}
      onclick={() => {
        value = null;
        selected = null;
        uploaded = false;
      }}>Quitar soporte: {label}</button
    >
  {/if}
  <div class="action-row">
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => (choosing = true)}>Elegir soporte: {label}</button
    >
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => uploader.open()}>Cargar soporte: {label}</button
    >
  </div>
  {#if choosing}<StagePicker
      api={documents}
      {caseId}
      {ondenied}
      {disabled}
      bind:busy={pickerBusy}
      onselected={(row) => select(row)}
      oncancel={() => (choosing = false)}
    />{/if}
  <p class="hint">
    La preparaci&#243;n admite PDF o DOCX. El soporte conserva su versi&#243;n y localizador
    exactos.
  </p>
</section>
<UploadDocument
  api={documents}
  {ondenied}
  {disabled}
  bind:this={uploader}
  bind:busy={uploadBusy}
  onuploaded={(row) => select(row, true)}
/>
