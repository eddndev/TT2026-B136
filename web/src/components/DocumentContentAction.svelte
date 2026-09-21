<script>
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import { download } from '../lib/documents.mjs';
  import { contentFilename } from '../lib/document-content.mjs';
  export let api,
    document,
    disabled = false,
    busy = '',
    error = '',
    message = '';
  export let ondenied = () => {};
  let alive = true;

  async function run() {
    if (busy || disabled) return;
    const reference = {
      caseId: document.case_id,
      id: document.id,
      version: document.version,
      digest: document.digest,
      name: document.name,
    };
    busy = 'content';
    error = '';
    message = '';
    try {
      const result = await api.content(reference.digest);
      if (
        !alive ||
        document.case_id !== reference.caseId ||
        document.id !== reference.id ||
        document.version !== reference.version ||
        document.digest !== reference.digest
      )
        return;
      download(result.blob, contentFilename(reference.name, reference.version));
      message = 'Descarga del archivo iniciada.';
    } catch (failure) {
      if (!alive) return;
      error = failure.message;
      if ([403, 404].includes(failure.status)) ondenied(failure);
    } finally {
      if (alive) busy = '';
    }
  }
  onDestroy(() => {
    alive = false;
    if (busy === 'content') busy = '';
  });
</script>

<button class="secondary" disabled={disabled || !!busy} onclick={run}>
  <Icon name="download" size={17} />{busy === 'content'
    ? 'Validando archivo...'
    : 'Descargar archivo'}
</button>
