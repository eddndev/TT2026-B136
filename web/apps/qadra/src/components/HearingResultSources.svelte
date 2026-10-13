<script>
  import { hearingKinds, hearingStatus, hearingTimeLabel } from '../lib/hearings.mjs';
  import { resultStatus } from '../lib/hearing-result-errors.mjs';
  export let anchor,
    continuation = null,
    onprevious = null,
    disabled = false;
</script>

<div class="case-comparison hearing-result-sources">
  <h3>Programaci&#243;n de origen</h3>
  <p>
    {hearingKinds[anchor.kind]?.label || 'Audiencia'} / {hearingStatus[anchor.status]} / Revisi&#243;n
    {anchor.revision}
  </p>
  <p>{hearingTimeLabel(anchor.scheduled_at)}</p>
  <details>
    <summary>Referencia exacta de programaci&#243;n</summary>
    <p>Audiencia <code>{anchor.hearing_id}</code></p>
    <p>Huella de valores <code>{anchor.values_digest}</code></p>
    <p>Recibo <code>{anchor.submission_digest}</code></p>
  </details>
  {#if continuation}<h3>Antecedente de continuaci&#243;n</h3>
    <p>{resultStatus[continuation.status]} / Revisi&#243;n {continuation.revision} exacta</p>
    <details>
      <summary>Referencia exacta del antecedente</summary>
      <p>Audiencia <code>{continuation.hearing_id}</code></p>
      <p>Registro <code>{continuation.result_id}</code></p>
      <p>Huella <code>{continuation.values_digest}</code></p>
      <p>Recibo <code>{continuation.submission_digest}</code></p>
    </details>
    {#if onprevious}<button
        type="button"
        class="secondary"
        {disabled}
        onclick={() => onprevious(continuation)}>Consultar antecedente exacto</button
      >{/if}
  {/if}
  <p class="hint">
    Fuentes hist&#243;ricas elegidas. Los cambios posteriores no sustituyen estas revisiones.
  </p>
</div>
