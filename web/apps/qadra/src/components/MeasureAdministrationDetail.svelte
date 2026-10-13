<script>
  import { onMount } from 'svelte';
  import MeasureRecordSummary from './MeasureRecordSummary.svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  import FactSources from './FactSources.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  import { stageLabels } from '../lib/case-stages.mjs';
  import { measureActions, measureKinds } from '../lib/measure-presentation.mjs';
  import '../styles/hearings.css';
  import '../styles/procedural-facts.css';
  export let value,
    onclose,
    record = null;
  let heading;
  const actions = {
    correct: 'Rectificacion de texto',
    entered_in_error: 'Registro marcado por error',
    replace_entered_in_error: 'Identidad registrada corregida mediante reemplazo',
  };
  $: capture = value.capture;
  $: review = capture.review;
  $: command = review.command;
  $: judicialHistory = [
    ...value.record_history.records.judicial.groups,
    ...value.record_history.decisions,
  ];
  function summary(row) {
    return {
      reference: {
        id: row.result.id,
        revision: row.result.revision,
        capture_digest: row.capture_digest,
      },
      validity: row.result.validity,
      last_action: row.result.last_action,
      record: { capture: row },
    };
  }
  onMount(() => heading?.focus());
</script>

<section class="card hearing-detail fact-detail" aria-label="Detalle de rectificacion">
  <div class="section-heading">
    <h2 tabindex="-1" bind:this={heading}>Rectificaci&#243;n administrativa registrada</h2>
    <button class="secondary" onclick={onclose}>Cerrar detalle de rectificacion</button>
  </div>
  {#if record}<p><strong>{record.title}</strong> / {record.reference}</p>{/if}
  {#if record?.administration?.administrative_status === 'closed'}
    <p class="notice">
      Expediente cerrado administrativamente. Consulta hist&#243;rica disponible.
    </p>
  {/if}
  <p class="hint">
    Recibo original de la operaci&#243;n administrativa. Conserva sus registros y antecedentes;
    puede haber revisiones posteriores de la medida.
  </p>
  <h3>{actions[command.action.kind]}</h3>
  <p class="case-multiline">{command.reason}</p>
  <p>
    Registrado por {review.actor.email} /
    <time datetime={capture.recorded_at}>{capture.recorded_at}</time>
  </p>
  <section class="case-comparison" aria-label="Registro seleccionado para rectificacion">
    <h3>Registro exacto seleccionado</h3>
    <p>Medida: <code>{command.target.id}</code> / Revisi&#243;n {command.target.revision}</p>
    <p>Captura seleccionada: <code>{command.target.capture_digest}</code></p>
  </section>
  <FactSources
    sources={{
      resolution: null,
      participants: [],
      hearing_results: [],
      direct_supports: [review.support],
    }}
  />
  <h3>Registros de esta operaci&#243;n</h3>
  {#each capture.records as row (row.result.id)}
    <MeasureRecordSummary value={summary(row)} />
    <details>
      <summary>Identidad exacta de este registro</summary>
      <ParticipantSubjectSummary record={row.result.sources.subject} />
    </details>
    {#if row.result.projection.supervisor}<FactSources
        sources={{
          resolution: null,
          participants: [
            {
              ...row.result.projection.supervisor.overview,
              values_digest: row.result.projection.supervisor.snapshot.values_digest,
            },
          ],
          hearing_results: [],
          direct_supports: [],
        }}
      />{/if}
    <details>
      <summary>Procedencia judicial conservada</summary>
      <p>Medida: <code>{row.result.id}</code></p>
      <p>Decisi&#243;n judicial de origen: <code>{row.result.judicial_origin.decision_id}</code></p>
      <p>
        Operaci&#243;n judicial de origen: <code>{row.result.judicial_origin.operation_id}</code>
      </p>
      <p>Ultima acci&#243;n judicial capturada: {measureActions[row.result.last_action]}</p>
      <p>
        Ultima medida judicial capturada: <code>{row.result.last_judicial.reference.id}</code>
        / Revisi&#243;n {row.result.last_judicial.reference.revision}
      </p>
      <p>Captura judicial: <code>{row.result.last_judicial.reference.capture_digest}</code></p>
      <p>Grupo judicial: <code>{row.result.last_judicial.owner.group_digest}</code></p>
      {#if row.result.record_root.kind === 'administrative'}
        <p>
          Ra&#237;z administrativa del registro: <code>{row.result.record_root.operation_id}</code>
        </p>
        <p>Identidad del registro de reemplazo: <code>{row.result.record_root.measure_id}</code></p>
      {:else}<p>El registro conserva su ra&#237;z judicial.</p>{/if}
      <p>
        Registro anterior: <code>{row.result.previous.id}</code>
        / Revisi&#243;n {row.result.previous.revision}
      </p>
      <p>Captura anterior: <code>{row.result.previous.capture_digest}</code></p>
    </details>
  {/each}
  {#if capture.replacement_link}
    <section class="case-comparison" aria-label="Enlace de reemplazo administrativo">
      <h3>Reemplazo registrado en la misma operaci&#243;n</h3>
      <p>
        Registro marcado por error: <code>{capture.replacement_link.entered_in_error.id}</code>
        / Revisi&#243;n {capture.replacement_link.entered_in_error.revision}
      </p>
      <p>
        Captura marcada: <code>{capture.replacement_link.entered_in_error.capture_digest}</code>
      </p>
      <p>
        Registro de reemplazo: <code>{capture.replacement_link.replacement.id}</code>
        / Revisi&#243;n {capture.replacement_link.replacement.revision}
      </p>
      <p>
        Captura de reemplazo: <code>{capture.replacement_link.replacement.capture_digest}</code>
      </p>
    </section>
  {/if}
  <details class="hearing-provenance">
    <summary>Antecedentes del registro</summary>
    <p class="hint">Capturas anteriores incluidas en el recibo consultado.</p>
    {#each judicialHistory as entry (entry.origin.operation_id)}
      <section class="case-comparison">
        <h4>Decisi&#243;n judicial antecedente</h4>
        <p>{entry.capture.review.command.values.authority}</p>
        <p>Decisi&#243;n: <code>{entry.origin.decision_id}</code></p>
        <p>Operaci&#243;n: <code>{entry.origin.operation_id}</code></p>
        {#each entry.capture.measures as previous (previous.result.id)}
          <p>
            {measureActions[previous.result.action]} / {measureKinds[previous.result.values.kind]}
            / {previous.result.projection.subject.display_name}
          </p>
          <p>
            Medida: <code>{previous.result.id}</code> / Revisi&#243;n {previous.result.revision}
          </p>
          <p>Captura: <code>{previous.capture_digest}</code></p>
        {/each}
        <p>Captura del grupo: <code>{entry.origin.group_digest}</code></p>
      </section>
    {/each}
    {#each value.record_history.records.administrative as entry (entry.origin.operation_id)}
      <section class="case-comparison">
        <h4>{actions[entry.capture.review.command.action.kind]}</h4>
        <p class="case-multiline">{entry.capture.review.command.reason}</p>
        <p>Operaci&#243;n: <code>{entry.origin.operation_id}</code></p>
        {#each entry.capture.records as previous (previous.result.id)}
          <p>
            {measureKinds[previous.result.values.kind]} /
            {previous.result.projection.subject.display_name}
          </p>
          <p>
            Medida: <code>{previous.result.id}</code> / Revisi&#243;n {previous.result.revision}
          </p>
          <p>Captura: <code>{previous.capture_digest}</code></p>
        {/each}
        <p>Captura administrativa: <code>{entry.origin.capture_digest}</code></p>
      </section>
    {/each}
    {#if !judicialHistory.length && !value.record_history.records.administrative.length}
      <p>Sin capturas antecedentes en este recibo.</p>
    {/if}
  </details>
  <details class="hearing-provenance">
    <summary>Contexto y recibo original</summary>
    <FactAdministrativeCapture
      value={review.context.administration}
      label="Administracion al registrar"
    />
    <p>
      Etapa: {stageLabels[review.context.stage.stage]} / Revisi&#243;n {review.context.stage
        .stage_revision}
    </p>
    <p>Contexto: <code>{review.context.context_digest}</code></p>
    <p>Expediente: <code>{value.origin.case_id}</code></p>
    <p>Operaci&#243;n: <code>{value.origin.operation_id}</code></p>
    <p>Huella del env&#237;o: <code>{value.origin.submission_digest}</code></p>
    <p>Huella de la revisi&#243;n: <code>{value.origin.review_digest}</code></p>
    <p>Captura administrativa: <code>{value.origin.capture_digest}</code></p>
  </details>
</section>
