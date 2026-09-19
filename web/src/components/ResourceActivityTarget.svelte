<script>
  import HearingValues from './HearingValues.svelte';
  import DeadlineTrackingSummary from './DeadlineTrackingSummary.svelte';
  import { hearingStatus } from '../lib/hearings.mjs';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  export let target,
    historical = true;
  $: record = target.record;
</script>

<p>{target.kind === 'hearing' ? 'Audiencia' : 'Plazo'} / Revisi&#243;n {record.revision}</p>
{#if target.kind === 'hearing'}
  <p class="badge">{hearingStatus[record.status]}</p>
  <HearingValues
    values={record.values}
    participants={record.participants}
    support={record.support}
  />
{:else}
  <h4>{record.definition.title}</h4>
  {#if historical}
    <p>
      C&#225;lculo conservado: {record.calculation.result.due_at
        ? deadlineInstantLabel(record.calculation.result.due_at)
        : 'Sin fecha calculada'}
    </p>
    <p class="hint">La fecha de esta captura no acredita un vencimiento operativo actual.</p>
  {:else}
    <p>
      {record.operational.due_at
        ? deadlineInstantLabel(record.operational.due_at)
        : 'Sin fecha operativa'}
    </p>
    <DeadlineTrackingSummary value={record} />
  {/if}
{/if}
<details>
  <summary>Identidad y autor de la actividad</summary>
  <p><code>{record.id}</code></p>
  <p>{record.recorded_by.email} / {record.recorded_at}</p>
  <p><code>{record.receipt.capture_digest || record.receipt.submission_digest}</code></p>
</details>
