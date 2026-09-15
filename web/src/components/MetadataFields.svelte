<script>
  import TagEditor from './TagEditor.svelte';
  import { normalizeMetadata } from '../lib/document-metadata.mjs';
  export let draft;
  export let prefix;
  export let disabled = false;
  let editor;
  export function values() {
    editor.flush();
    return normalizeMetadata(draft);
  }
  export function reset() {
    editor?.reset();
  }
</script>

<fieldset class="metadata-fields" {disabled}>
  <legend>Organizaci&#243;n del documento (opcional)</legend>
  <label>Tipo de documento (opcional)<input bind:value={draft.document_type} /></label>
  <label>Clasificaci&#243;n (opcional)<input bind:value={draft.classification} /></label>
  <p class="hint">
    Texto libre. Hasta 80 caracteres por campo. El acceso lo determina el expediente.
  </p>
  <TagEditor bind:this={editor} bind:tags={draft.tags} {disabled} {prefix} />
</fieldset>
