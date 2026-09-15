<script>
  import { normalizeTag } from '../lib/document-metadata.mjs';
  export let prefix;
  export let tags = [];
  export let disabled = false;
  let pending = '';
  let error = '';
  let message = '';
  let input;
  export function flush() {
    if (!pending) return;
    try {
      const tag = normalizeTag(pending);
      if (!tags.includes(tag)) {
        if (tags.length >= 20) throw new Error('Usa hasta 20 etiquetas.');
        tags = [...tags, tag];
      }
      pending = '';
      error = '';
      message = 'Etiqueta agregada.';
    } catch (failure) {
      error = failure.message;
      input?.focus();
      failure.field = 'tag';
      throw failure;
    }
  }
  function add() {
    try {
      flush();
    } catch {}
  }
  export function reset() {
    pending = '';
    error = '';
    message = '';
  }
</script>

<div class="metadata-tags-editor">
  <label
    >Nueva etiqueta<input
      bind:this={input}
      bind:value={pending}
      {disabled}
      aria-invalid={!!error}
      aria-describedby={error ? `${prefix}-tag-error` : undefined}
      onkeydown={(event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          add();
        }
      }}
    /></label
  >
  <button type="button" class="secondary" {disabled} onclick={add}>Agregar etiqueta</button>
</div>
<p class="hint">
  Agrega una etiqueta por vez. Puede contener comas y acentos. Hasta 20 etiquetas de 40 caracteres.
</p>
<ul class="metadata-tags" aria-label="Etiquetas del documento">
  {#each tags as tag, index}<li class="badge">
      <span>{tag}</span><button
        type="button"
        class="text-button"
        {disabled}
        aria-label={`Quitar etiqueta: ${tag}`}
        onclick={() => {
          tags = tags.filter((_, i) => i !== index);
          message = 'Etiqueta retirada.';
        }}>Quitar</button
      >
    </li>{/each}
</ul>
{#if error}<p id={`${prefix}-tag-error`} class="notice error" role="alert">{error}</p>{/if}
<span class="sr-only" role="status">{message}</span>
