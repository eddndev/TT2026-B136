<script>
  export let draft,
    disabled = false;
  function move(index, step) {
    const rows = [...draft.agreements],
      next = index + step;
    [rows[index], rows[next]] = [rows[next], rows[index]];
    draft.agreements = rows;
  }
</script>

<details class="hearing-result-optional">
  <summary>Acuerdos declarados ({draft.agreements.length}/16)</summary>
  <p class="hint">
    Transcribe lo comunicado. La procedencia es com&#250;n al relato; no se afirma unanimidad,
    firmeza ni c&#243;mputo de plazos.
  </p>
  {#each draft.agreements as item, index (item.id)}<div class="case-comparison">
      <label
        >Texto del acuerdo {index + 1}<textarea rows="3" bind:value={item.text} {disabled}
        ></textarea></label
      >
      <details><summary>Identificador del acuerdo</summary><code>{item.id}</code></details>
      <div class="action-row">
        <button
          type="button"
          class="secondary"
          disabled={disabled || index === 0}
          onclick={() => move(index, -1)}>Subir acuerdo {index + 1}</button
        >
        <button
          type="button"
          class="secondary"
          disabled={disabled || index === draft.agreements.length - 1}
          onclick={() => move(index, 1)}>Bajar acuerdo {index + 1}</button
        >
        <button
          type="button"
          class="text-button"
          {disabled}
          onclick={() => (draft.agreements = draft.agreements.filter((row) => row.id !== item.id))}
          >Quitar acuerdo {index + 1}</button
        >
      </div>
    </div>{/each}
  <button
    type="button"
    class="secondary"
    disabled={disabled || draft.agreements.length >= 16}
    onclick={() =>
      (draft.agreements = [...draft.agreements, { id: crypto.randomUUID(), text: '' }])}
    >Agregar acuerdo declarado</button
  >
</details>
