<script>
  import { hearingModalities, hearingTimeLabel } from '../lib/hearings.mjs';
  import { resourceHearingKinds } from '../lib/resource-hearing-values.mjs';
  export let values;
</script>

<dl class="case-values">
  <div>
    <dt>Tipo de audiencia de recurso</dt>
    <dd>{resourceHearingKinds[values.kind]}</dd>
  </div>
  <div>
    <dt>Programaci&#243;n declarada</dt>
    <dd>{hearingTimeLabel(values.scheduled_at)}</dd>
  </div>
  <div>
    <dt>Modalidad</dt>
    <dd>{hearingModalities[values.modality]}</dd>
  </div>
  <div>
    <dt>Sede o conexi&#243;n</dt>
    <dd>{values.venue}</dd>
  </div>
  {#if values.note}<div class="case-value-wide">
      <dt>Nota declarada</dt>
      <dd class="case-multiline">{values.note}</dd>
    </div>{/if}
</dl>
<section class="case-comparison" aria-label="Base de se&#241;alamiento">
  <h3>Base de se&#241;alamiento declarada</h3>
  <p class="case-multiline">{values.scheduling_basis.statement}</p>
  <p>Soporte exacto / Versi&#243;n {values.scheduling_basis.support.version}</p>
  <details>
    <summary>Referencia exacta del soporte de se&#241;alamiento</summary>
    <p>Documento: <code>{values.scheduling_basis.support.document_id}</code></p>
    <p>Huella: <code>{values.scheduling_basis.support.digest}</code></p>
  </details>
</section>
