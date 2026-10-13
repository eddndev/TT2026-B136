<script>
  export let value;
</script>

{#if value}<p class="hint participant-provenance">
    Documento {value.document_id} / versi&#243;n {value.version}<br />SHA-256: {value.digest}<br
    />P&#225;gina o secci&#243;n: {value.locator}
  </p>{/if}
