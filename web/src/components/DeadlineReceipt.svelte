<script>
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import {
    deadlineAuthorLabel,
    deadlineDependencies,
    deadlineFamilies,
  } from './deadline-view-labels.mjs';
  export let value,
    compact = false;
  $: author = value.recorded_by || value.author;
  $: receipt = value.receipt || value;
  $: version = value.receipt?.version || value.receipt_version;
  $: cause = version.kind === 'v2' ? version.cause : null;
</script>

<div class="deadline-receipt">
  <p>
    {deadlineAuthorLabel(author)}{#if value.recorded_at}
      / {deadlineInstantLabel(value.recorded_at)}{/if}
  </p>
  <p>Recibo {version.kind === 'v2' ? 'V2' : 'V1'}</p>
  {#if cause?.kind === 'legacy_bootstrap'}
    <p>Inicio del seguimiento de un registro anterior.</p>
  {:else if cause?.kind === 'source_event'}
    <p>
      Cambio observado: {deadlineDependencies[cause.event.family] ||
        deadlineFamilies[cause.event.family]}
      / Revisi&#243;n {cause.event.revision} / Secuencia {cause.event.sequence}
    </p>
  {/if}
  {#if !compact}
    {#if author.kind === 'technical'}<p>Pol&#237;tica t&#233;cnica: {author.policy_version}</p>{/if}
    {#if cause}<p>Trabajo: <code>{cause.job_id}</code></p>{/if}
    {#if cause?.kind === 'source_event'}
      <p>Ra&#237;z del evento: <code>{cause.event.source_id}</code></p>
      {#if cause.event.hearing_id}<p>Audiencia: <code>{cause.event.hearing_id}</code></p>{/if}
      <p>Operaci&#243;n de origen: <code>{cause.event.operation_id}</code></p>
    {/if}
    <p>Operaci&#243;n: <code>{value.receipt?.operation_id || value.command.operation_id}</code></p>
    <p>Confirmaci&#243;n: <code>{receipt.submission_digest}</code></p>
    <p>Contenido revisado: <code>{receipt.review_digest}</code></p>
    <p>Captura: <code>{receipt.capture_digest}</code></p>
    {#if version.kind === 'v2'}
      <p>Observaciones: <code>{version.observations_digest}</code></p>
      {#if version.predecessor}<p>
          Confirmaci&#243;n anterior: <code>{version.predecessor.submission_digest}</code>
        </p>
        <p>Captura anterior: <code>{version.predecessor.capture_digest}</code></p>{/if}
    {/if}
  {/if}
</div>
