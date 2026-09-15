<script>
  export let record;
</script>

{#if record}
  <p class="stage-support-name">
    <strong>{record.name || 'Documento seleccionado'}</strong> / Versi&#243;n {record.version}
  </p>
  <details>
    <summary>Digest del contenido seleccionado</summary><code>{record.digest}</code>
  </details>
{/if}
