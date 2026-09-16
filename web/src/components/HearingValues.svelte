<script>
  import HearingParticipants from './HearingParticipants.svelte';
  import { hearingKinds, hearingModalities, hearingTimeLabel } from '../lib/hearings.mjs';
  export let values,
    participants = [],
    support = null;
</script>

<dl class="case-values">
  <div>
    <dt>Tipo de audiencia</dt>
    <dd>{hearingKinds[values.kind]?.label || values.kind}</dd>
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
{#if values.conviction_basis}<div class="case-comparison hearing-antecedent-summary">
    <h3>Antecedente de condena declarado</h3>
    <p class="case-multiline">{values.conviction_basis.statement}</p>
    <p>
      {support?.name || 'Soporte documental'} / Versi&#243;n {values.conviction_basis.support
        .version}
    </p>
    <details>
      <summary>Referencia exacta del soporte</summary>
      <p><code>{values.conviction_basis.support.document_id}</code></p>
      <p><code>{values.conviction_basis.support.digest}</code></p>
    </details>
    <p class="hint">
      Declaraci&#243;n del operador con soporte vinculado; no es una validaci&#243;n judicial del
      resultado.
    </p>
  </div>{/if}
<h3>Participantes vinculados</h3>
<HearingParticipants rows={participants} />
