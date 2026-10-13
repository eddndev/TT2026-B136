<script>
  import { onMount } from 'svelte';
  import MeasureDecisionReview from './MeasureDecisionReview.svelte';
  import { measureActions, measureKinds, measureTimeLabel } from '../lib/measure-presentation.mjs';
  import '../styles/hearings.css';
  import '../styles/procedural-facts.css';
  export let value, record, onclose;
  let heading;
  $: group = value.group;
  $: judicialHistory =
    value.family === 'g1'
      ? value.measure_history.groups
      : [...value.record_history.records.judicial.groups, ...value.record_history.decisions];
  $: administrativeHistory =
    value.family === 'g1' ? [] : value.record_history.records.administrative;
  const administrativeActions = {
    correct: 'Rectificacion de texto',
    entered_in_error: 'Registro marcado por error',
    replace_entered_in_error: 'Registro marcado por error y reemplazado',
  };
  onMount(() => heading?.focus());
</script>

<section class="card hearing-detail fact-detail" aria-label="Detalle de decision cautelar">
  <div class="section-heading">
    <h2 tabindex="-1" bind:this={heading}>Decisi&#243;n cautelar declarada</h2>
    <button class="secondary" onclick={onclose}>Cerrar detalle de decision</button>
  </div>
  <p><strong>{record.title}</strong> / {record.reference}</p>
  {#if record.administration?.administrative_status === 'closed'}
    <p class="notice">
      Expediente cerrado administrativamente. Consulta hist&#243;rica disponible.
    </p>
  {/if}
  <p class="hint">
    Consulta del recibo original de esta operaci&#243;n. Las medidas pueden tener revisiones
    posteriores; este detalle conserva lo registrado aqu&#237;.
  </p>
  <p>
    Registrado por {group.review.actor.email} /
    <time datetime={group.recorded_at}>{group.recorded_at}</time>
  </p>
  <MeasureDecisionReview value={{ family: value.family, review: group.review }} confirmed={true} />
  <details class="hearing-provenance">
    <summary>Historia antecedente de la decision</summary>
    <p class="hint">Capturas anteriores incluidas como evidencia del recibo consultado.</p>
    {#if !judicialHistory.length && !administrativeHistory.length}
      <p>Sin capturas antecedentes en este recibo.</p>
    {/if}
    {#each judicialHistory as entry (entry.origin.operation_id)}
      <section class="case-comparison">
        <h3>Decisi&#243;n antecedente</h3>
        <p>{entry.capture.review.command.values.authority}</p>
        <p>{measureTimeLabel(entry.capture.review.command.values.declared_at)}</p>
        <p class="case-multiline">{entry.capture.review.command.values.justification}</p>
        {#if entry.capture.review.command.outcome.kind === 'no_measure_change'}
          <p class="case-multiline">{entry.capture.review.command.outcome.statement}</p>
        {:else}
          {#each entry.capture.review.results as result (result.id)}
            <p>
              {measureActions[result.action]} / {measureKinds[result.values.kind]} /
              {result.projection.subject.display_name} / Revisi&#243;n {result.revision}
            </p>
            <p class="case-multiline">{result.values.conditions}</p>
          {/each}
        {/if}
        <p>
          {entry.capture.review.actor.email} /
          <time datetime={entry.capture.recorded_at}>{entry.capture.recorded_at}</time>
        </p>
        <details>
          <summary>Referencia del grupo antecedente</summary>
          <p>Decisi&#243;n: <code>{entry.origin.decision_id}</code></p>
          <p>Operaci&#243;n: <code>{entry.origin.operation_id}</code></p>
          <p>Captura del grupo: <code>{entry.origin.group_digest}</code></p>
        </details>
      </section>
    {/each}
    {#each administrativeHistory as entry (entry.origin.operation_id)}
      <section class="case-comparison">
        <h3>{administrativeActions[entry.capture.review.command.action.kind]}</h3>
        <p class="case-multiline">{entry.capture.review.command.reason}</p>
        {#each entry.capture.records as capture (capture.result.id)}
          <p>
            {measureKinds[capture.result.values.kind]} /
            {capture.result.projection.subject.display_name} / Revisi&#243;n {capture.result
              .revision}
          </p>
          <p class="case-multiline">{capture.result.values.conditions}</p>
          {#if capture.result.validity === 'entered_in_error'}<p>
              Captura registrada por error.
            </p>{/if}
        {/each}
        <p>
          {entry.capture.review.actor.email} /
          <time datetime={entry.capture.recorded_at}>{entry.capture.recorded_at}</time>
        </p>
        <details>
          <summary>Referencia de la rectificaci&#243;n antecedente</summary>
          <p>Operaci&#243;n: <code>{entry.origin.operation_id}</code></p>
          <p>Captura: <code>{entry.origin.capture_digest}</code></p>
        </details>
      </section>
    {/each}
  </details>
  <details class="hearing-provenance">
    <summary>Autor y recibo original</summary>
    <p>
      {group.review.actor.email} /
      <time datetime={group.recorded_at}>{group.recorded_at}</time>
    </p>
    <p>Familia conservada: <code>{value.family}</code></p>
    <p>Decisi&#243;n: <code>{value.origin.decision_id}</code></p>
    <p>Operaci&#243;n: <code>{value.origin.operation_id}</code></p>
    <p>Huella del env&#237;o: <code>{value.origin.submission_digest}</code></p>
    <p>Huella de la revisi&#243;n: <code>{value.origin.review_digest}</code></p>
    <p>Captura de la decisi&#243;n: <code>{value.origin.decision_digest}</code></p>
    <p>Captura del grupo: <code>{value.origin.group_digest}</code></p>
    {#each group.measures as capture (capture.result.id)}
      <p>
        Medida <code>{capture.result.id}</code> / Revisi&#243;n {capture.result.revision} / Captura
        <code>{capture.capture_digest}</code>
      </p>
    {/each}
  </details>
</section>
