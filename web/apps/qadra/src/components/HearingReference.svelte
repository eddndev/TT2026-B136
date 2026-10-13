<script>
  export let row,
    label = 'Referencia exacta del participante';
</script>

<details class="hearing-reference">
  <summary>{label}</summary>
  <p>Ficha <code>{row.id}</code> / Revisi&#243;n {row.revision}</p>
  {#if row.values_digest}<p>Huella de ficha: <code>{row.values_digest}</code></p>{/if}
  {#if row.subject}<p>
      Identidad <code>{row.subject.id}</code> / Revisi&#243;n {row.subject.revision}
    </p>
    {#if row.subject.values_digest}<p>
        Huella de identidad: <code>{row.subject.values_digest}</code>
      </p>{/if}
  {/if}
</details>
