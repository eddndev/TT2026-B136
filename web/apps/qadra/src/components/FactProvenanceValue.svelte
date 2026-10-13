<script>
  export let value,
    label = 'Procedencia';
</script>

<section class="case-comparison fact-provenance-value">
  <h4>{label}</h4>
  {#if value.kind === 'operator_note'}<p>Nota del operador</p>
    <p class="case-multiline">{value.note}</p>
  {:else if value.kind === 'external_reference'}<p>Referencia externa</p>
    <p class="case-multiline">{value.reference}</p>
  {:else if value.kind === 'hearing_result'}<p>
      Resultado de audiencia / Revisi&#243;n {value.reference.revision}
    </p>
    <p class="case-multiline">Localizador: {value.locator}</p>
    <details>
      <summary>Referencia exacta de procedencia</summary>
      <p>Audiencia: <code>{value.reference.hearing_id}</code></p>
      <p>Resultado: <code>{value.reference.result_id}</code></p>
      <p>
        Acuerdo: {#if value.reference.agreement_id}<code>{value.reference.agreement_id}</code
          >{:else}No especificado{/if}
      </p>
    </details>
  {/if}
  {#if value.support}<p>Soporte documental / Versi&#243;n {value.support.version}</p>
    <p class="case-multiline">Localizador documental: {value.support.locator}</p>
    <details>
      <summary>Referencia exacta del soporte</summary>
      <p><code>{value.support.document_id}</code></p>
      <p>Huella: <code>{value.support.digest}</code></p>
    </details>
  {:else}<p class="hint">Sin soporte documental directo en esta procedencia.</p>{/if}
</section>
