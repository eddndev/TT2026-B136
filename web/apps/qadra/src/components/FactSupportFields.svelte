<script>
  import { getContext, onDestroy } from 'svelte';
  import { createStageSupportDraft, stageSupportKey } from '../lib/stage-support-draft.mjs';
  import StagePicker from './StagePicker.svelte';
  import StageSupportSummary from './StageSupportSummary.svelte';
  import UploadDocument from './UploadDocument.svelte';
  export let value = null,
    api,
    caseId,
    label,
    ondenied,
    disabled = false,
    pending = false,
    required = false,
    draftContext = null,
    ondiscard = () => {};
  const documents = api.caseDocuments(caseId);
  let choosing = false,
    pickerBusy = false,
    uploadBusy = false,
    uploader,
    selected = null,
    uploaded = false;
  const session = getContext('session-drafts');
  let recovery = null,
    recoveryKey = null,
    picker,
    pickerDraft = null;
  let preparing = false,
    error = '',
    alive = true,
    generation = 0;
  $: configure(draftContext);
  $: pending = preparing || pickerBusy || uploadBusy;
  $: childContext =
    draftContext && recovery
      ? {
          ...draftContext,
          ownerDraftKey: recovery.key,
          fieldPath: ['upload'],
          rowId: null,
          canApply: admitted,
        }
      : null;
  function configure(context) {
    const key = context && session ? stageSupportKey(context) : null;
    if (key === recoveryKey) return;
    recovery?.dispose();
    generation++;
    recoveryKey = key;
    pickerDraft = null;
    choosing = false;
    recovery = key
      ? createStageSupportDraft({
          session,
          context: () => draftContext,
          capture: () => ({ picker: picker?.captureDraft() ?? pickerDraft }),
        })
      : null;
  }
  function admitted() {
    return alive && (!recovery || recovery.admitted());
  }
  async function prepare(operation) {
    if (pending || disabled || !admitted()) return;
    if (!recovery) return operation();
    const ticket = generation,
      valid = () => admitted() && ticket === generation;
    preparing = true;
    error = '';
    try {
      if (recovery?.pending()) {
        const result = await recovery.restore((value) => {
          pickerDraft = value.picker;
        });
        if (!valid()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar el selector de soporte.');
      } else if (recovery) {
        if (!(await recovery.authorize()) || !valid()) return;
        recovery.register();
      }
      if (valid()) await operation();
    } catch (failure) {
      if (valid()) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive && ticket === generation) preparing = false;
    }
  }
  function closePicker() {
    pickerDraft = picker?.captureDraft() ?? pickerDraft;
    choosing = false;
  }
  function remove() {
    if (pending || disabled || !admitted()) return;
    recovery?.close();
    ondiscard();
    pickerDraft = null;
    choosing = false;
    value = null;
    selected = null;
    uploaded = false;
  }
  function select(row, created = false) {
    if (!admitted() || row.case_id !== caseId) return;
    selected = row;
    uploaded = created;
    value = {
      document_id: row.id,
      version: row.version,
      digest: row.digest,
      locator: value?.locator || '',
    };
    closePicker();
  }
  onDestroy(() => {
    alive = false;
    generation++;
    recovery?.dispose();
    documents.dispose();
    pending = false;
  });
</script>

<section class="fact-support-fields" aria-label={`Soporte: ${label}`}>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <h4>Soporte documental de {label} ({required ? 'obligatorio' : 'opcional'})</h4>
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
    <button type="button" class="text-button" disabled={disabled || pending} onclick={remove}
      >Quitar soporte: {label}</button
    >
  {/if}
  <div class="action-row">
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() =>
        prepare(() => {
          choosing = true;
        })}>Elegir soporte: {label}</button
    >
    <button
      type="button"
      class="secondary"
      disabled={disabled || pending}
      onclick={() => prepare(() => uploader.open())}>Cargar soporte: {label}</button
    >
  </div>
  {#if choosing}<StagePicker
      api={documents}
      {caseId}
      {ondenied}
      {disabled}
      bind:this={picker}
      draft={pickerDraft}
      canApply={admitted}
      bind:busy={pickerBusy}
      onselected={(row) => select(row)}
      oncancel={closePicker}
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
  draftContext={childContext}
  bind:this={uploader}
  bind:busy={uploadBusy}
  onuploaded={(row) => select(row, true)}
/>
