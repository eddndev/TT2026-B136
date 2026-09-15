<script>
  import { offenseValue } from '../lib/case-administration.mjs';
  export let offenses = [];
  export let disabled = false;
  let pending = '',
    error = '',
    input;
  export function flush() {
    if (!pending) return;
    try {
      const value = offenseValue(pending);
      if (offenses.includes(value))
        throw new Error('Esta descripci\u00f3n ya est\u00e1 en la lista.');
      if (offenses.length >= 8) throw new Error('Usa hasta 8 descripciones de delito.');
      offenses = [...offenses, value];
      pending = '';
      error = '';
    } catch (failure) {
      error = failure.message;
      input?.focus();
      failure.field = 'offenses';
      throw failure;
    }
  }
  function add() {
    try {
      flush();
    } catch {}
  }
</script>

<fieldset class="case-offenses">
  <legend>Delitos registrados</legend>
  <div class="metadata-tags-editor">
    <label
      >Nueva descripci&#243;n de delito<input
        bind:this={input}
        bind:value={pending}
        {disabled}
        aria-invalid={!!error}
        onkeydown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            add();
          }
        }}
      /></label
    >
    <button type="button" class="secondary" {disabled} onclick={add}
      >Agregar descripci&#243;n</button
    >
  </div>
  <p class="hint">
    Agrega de 1 a 8 descripciones manuales de hasta 120 caracteres. Las comas forman parte del
    texto.
  </p>
  <ol class="case-offense-list">
    {#each offenses as value, index}<li>
        <span>{value}</span><button
          type="button"
          class="text-button"
          {disabled}
          aria-label={`Quitar descripci\u00f3n: ${value}`}
          onclick={() => (offenses = offenses.filter((_, i) => i !== index))}>Quitar</button
        >
      </li>{/each}
  </ol>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</fieldset>
