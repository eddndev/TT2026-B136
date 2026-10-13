<script>
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import {
    deadlineReviewLabels,
    deadlineDependencies,
    deadlinePolicyLabels,
    deadlineFreshnessLabels,
    deadlineReviewReasonLabel,
    deadlineOperationalLabel,
  } from './deadline-view-labels.mjs';
  export let value,
    historical = false,
    prepared = false;
  $: tracking = value.tracking;
  $: review = tracking?.review.state || 'legacy_undeclared';
  $: source = value.definition.input.selection.source;
  $: selected = {
    profile: value.definition.profile,
    source: source.kind === 'known' ? source.value : null,
    calendar: value.definition.input.calendar,
  };
  $: observed = (role) => tracking?.observations.entries.find((row) => row.role === role);
</script>

<section class="case-comparison deadline-tracking" aria-label="Seguimiento del plazo">
  <div class="section-heading">
    <h3>{prepared ? 'Seguimiento preparado' : 'Seguimiento de esta captura'}</h3>
    <span class="badge" class:warning={review !== 'accepted'} class:info={review === 'accepted'}
      >{deadlineReviewLabels[review]}</span
    >
  </div>
  {#if prepared}<p>
      El borrador conserva el resultado propuesto; a&#250;n requiere confirmaci&#243;n.
    </p>
  {:else}
    <p><strong>Vencimiento para seguimiento</strong></p>
    <p class="deadline-operational" class:deadline-due={!historical && !!value.operational.due_at}>
      {deadlineOperationalLabel(value.operational, value.status, review, historical)}
    </p>
    <p>
      {historical
        ? 'Consulta hist\u00f3rica exacta'
        : deadlineFreshnessLabels[value.operational.freshness]}
      {#if !historical && value.operational.checked_at}
        / {deadlineInstantLabel(value.operational.checked_at)}{/if}
    </p>
    {#if !historical && value.operational.changed_dependencies.length}<p>
        Dependencias: {value.operational.changed_dependencies
          .map((item) => deadlineDependencies[item])
          .join(', ')}
      </p>{/if}
  {/if}
  {#if tracking?.review.reasons.length}
    <h4>Qu&#233; requiere revisi&#243;n</h4>
    <ul>
      {#each tracking.review.reasons as reason}<li>{deadlineReviewReasonLabel(reason)}</li>{/each}
    </ul>
  {/if}
  {#if tracking}
    <dl class="case-values">
      {#each ['profile', 'source', 'calendar'] as dependency}
        <div>
          <dt>{deadlineDependencies[dependency]}</dt>
          <dd>
            {selected[dependency]
              ? deadlinePolicyLabels[tracking.policies[dependency]]
              : 'Sin dependencia seleccionada'}
          </dd>
        </div>
      {/each}
    </dl>
    <details>
      <summary>Selecciones y observaciones de esta captura</summary>
      {#each ['profile', 'source', 'calendar'] as dependency}
        {@const entry = observed(dependency)}
        <section class="case-comparison">
          <h4>{deadlineDependencies[dependency]}</h4>
          {#if selected[dependency]}<p>
              Seleccionada para el c&#225;lculo: revisi&#243;n {selected[dependency].revision}
            </p>
          {:else}<p>Sin dependencia seleccionada.</p>{/if}
          {#if entry}
            <p>Observada en esta captura: revisi&#243;n {entry.revision}</p>
            <p>Identidad: <code>{entry.id}</code></p>
            {#if entry.hearing_id}<p>Audiencia: <code>{entry.hearing_id}</code></p>{/if}
            {#if dependency === 'source' && source.value?.family === 'hearing_result'}
              <p>
                Acuerdo: {source.value.agreement_id || 'Resultado completo, sin acuerdo especifico'}
              </p>
            {/if}
            {#if dependency === 'source' && source.value?.family === 'notification'}
              <p>
                Resoluci&#243;n vinculada a la selecci&#243;n: revisi&#243;n {source.value
                  .resolution.revision}
              </p>
              <p>
                Resoluci&#243;n vinculada a la notificaci&#243;n observada: revisi&#243;n {entry
                  .parent_resolution.revision}
              </p>
              {@const parent = observed('notification_parent')}
              {#if parent}<p>
                  Resoluci&#243;n observada de forma independiente: revisi&#243;n {parent.revision}
                </p>
                <code>{parent.id}</code>
              {:else}<p>Sin observaci&#243;n independiente de la resoluci&#243;n.</p>{/if}
            {/if}
            <p>Confirmaci&#243;n de dependencia: <code>{entry.submission_digest}</code></p>
            <p>Evidencia observada: <code>{entry.evidence_digest}</code></p>
          {:else}<p>Sin observaci&#243;n de esta dependencia.</p>{/if}
        </section>
      {/each}
      <p>
        Expediente observado: {tracking.administration.title} /
        {tracking.administration.status === 'closed' ? 'Cerrado' : 'Activo'} /
        {tracking.administration.kind === 'recorded'
          ? `Revision ${tracking.administration.revision}`
          : 'Ficha original sin revision'}
      </p>
    </details>
  {:else}<p>
      Esta captura V1 conserva su c&#225;lculo; el seguimiento a&#250;n no est&#225; declarado.
    </p>{/if}
</section>
