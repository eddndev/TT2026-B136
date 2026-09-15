<script>
  import { stageLabels, stageSupportFields } from '../lib/case-stages.mjs';
  import { dateLabel } from '../lib/stage-dates.mjs';
  import StageSupportSummary from './StageSupportSummary.svelte';
  export let values,
    supports = [];
  const times = {
    known_at: 'Etapa conocida',
    accusation_declared_at: 'Acusaci\u00f3n declarada',
    opening_order_issued_at: 'Emisi\u00f3n del auto',
    received_at: 'Recepci\u00f3n por el tribunal',
  };
  const texts = {
    reason: 'Motivo de adopci\u00f3n',
    receiving_court: 'Tribunal receptor',
    receipt_reference: 'Referencia de recepci\u00f3n',
    note: 'Nota',
  };
  function snapshot(ref) {
    return (
      supports.find(
        (s) => (s.document_id || s.id) === ref.document_id && s.version === ref.version,
      ) || ref
    );
  }
</script>

<dl class="case-values">
  {#if values.stage}<div>
      <dt>Etapa conocida</dt>
      <dd>{stageLabels[values.stage]}</dd>
    </div>{/if}
  {#each Object.entries(times) as [key, label]}{#if values[key]}<div>
        <dt>{label}</dt>
        <dd>{dateLabel(values[key])}</dd>
      </div>{/if}{/each}
  {#each Object.entries(texts) as [key, label]}{#if values[key]}<div>
        <dt>{label}</dt>
        <dd class:case-multiline={key === 'note' || key === 'reason'}>{values[key]}</dd>
      </div>{/if}{/each}
</dl>
{#each Object.entries(stageSupportFields) as [key, label]}{#if values[key]}
    <div class="stage-support-summary">
      <h4>{label}</h4>
      <StageSupportSummary record={snapshot(values[key])} />
    </div>
  {/if}{/each}
