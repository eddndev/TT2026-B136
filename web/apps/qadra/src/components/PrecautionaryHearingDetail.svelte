<script>
  import { onMount } from 'svelte';
  import FactSources from './FactSources.svelte';
  import { hearingModalities, hearingStatus, hearingTimeLabel } from '../lib/hearings.mjs';
  import {
    precautionaryHearingPurposes,
    precautionaryHearingActions,
  } from '../lib/precautionary-hearing-presentation.mjs';
  import '../styles/hearings.css';
  import '../styles/procedural-facts.css';
  export let value, caseRecord, onclose;
  let heading;
  $: capture = value.capture;
  $: review = capture.review;
  $: values = review.resolved_values;
  onMount(() => heading?.focus());
</script>

<section class="card hearing-detail fact-detail" aria-label="Detalle de audiencia cautelar">
  <div class="section-heading">
    <h2 tabindex="-1" bind:this={heading}>{precautionaryHearingPurposes[values.purpose]}</h2>
    <button class="secondary" onclick={onclose}>Cerrar detalle de audiencia</button>
  </div>
  <p><strong>{caseRecord.title}</strong> / {caseRecord.reference}</p>
  {#if caseRecord.administration?.administrative_status === 'closed'}
    <p class="notice">
      Expediente cerrado administrativamente. Consulta hist&#243;rica disponible.
    </p>
  {/if}
  <p class="hint">Revisi&#243;n exacta consultada: {review.result_revision}</p>
  <p>Estado en esta revisi&#243;n: <strong>{hearingStatus[review.status]}</strong></p>
  <p class="hint">
    La consulta conserva las fuentes y la historia hasta esta revisi&#243;n. Puede haber cambios
    posteriores.
  </p>
  <dl class="case-values">
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
  <section class="case-comparison" aria-label="Base de senalamiento">
    <h3>Base de se&#241;alamiento declarada</h3>
    <p class="case-multiline">{values.scheduling_basis.statement}</p>
    <p class="case-multiline">Localizador: {values.scheduling_basis.locator}</p>
  </section>
  {#if values.review_targets.length}
    <section class="case-comparison" aria-label="Medidas seleccionadas para revision">
      <h3>Medidas seleccionadas para revisi&#243;n</h3>
      {#each values.review_targets as target (target.id)}
        <details>
          <summary>Medida / Revisi&#243;n {target.revision}</summary>
          <p>Identidad: <code>{target.id}</code></p>
          <p>Captura: <code>{target.capture_digest}</code></p>
        </details>
      {/each}
    </section>
  {/if}
  <FactSources
    sources={{
      resolution: null,
      participants: review.participants.map((person) => person.overview),
      hearing_results: [],
      direct_supports: [review.sources.support],
    }}
  />
  <details class="hearing-provenance">
    <summary>Historia de la convocatoria</summary>
    {#each value.history.captures as entry (entry.review.result_revision)}
      <section class="case-comparison">
        <h3>
          Revisi&#243;n {entry.review.result_revision} / {precautionaryHearingActions[
            entry.review.command.change.action
          ]}
        </h3>
        <p>
          {hearingStatus[entry.review.status]} / {hearingTimeLabel(
            entry.review.resolved_values.scheduled_at,
          )}
        </p>
        <p>{entry.review.resolved_values.venue}</p>
        {#if entry.review.command.change.reason}<p class="case-multiline">
            {entry.review.command.change.reason}
          </p>{/if}
        <p>
          {entry.review.actor.email} / <time datetime={entry.recorded_at}>{entry.recorded_at}</time>
        </p>
        <p>Captura: <code>{entry.capture_digest}</code></p>
      </section>
    {/each}
  </details>
  <details class="hearing-provenance">
    <summary>Autor y recibo original</summary>
    <p>{review.actor.email} / <time datetime={capture.recorded_at}>{capture.recorded_at}</time></p>
    <p>Audiencia: <code>{review.command.hearing_id}</code></p>
    <p>Operaci&#243;n: <code>{review.command.operation_id}</code></p>
    <p>Recibo: <code>{review.submission_digest}</code></p>
    <p>Captura: <code>{capture.capture_digest}</code></p>
  </details>
</section>
