<script>
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import {
    deadlineBlockLabel,
    deadlineRuleLabel,
    deadlineRequirementLabel,
  } from './deadline-view-labels.mjs';
  import DeadlineTrace from './DeadlineTrace.svelte';
  export let calculation,
    compact = false;
  let traceOpen = false;
  $: result = calculation.result;
</script>

<section class="deadline-calculation" aria-label="Resultado del plazo">
  <div class="section-heading">
    <h3>{result.due_at ? 'Vencimiento calculado' : 'Sin vencimiento fijado'}</h3>
    <span class="badge" class:warning={result.blocks.length > 0} class:info={!!result.due_at}>
      {result.blocks.length ? 'Requiere datos o revisi\u00f3n' : 'Evaluado'}
    </span>
  </div>
  {#if result.due_at}<p class="deadline-due">{deadlineInstantLabel(result.due_at)}</p>{/if}
  <p>
    <strong>{calculation.profile.title}</strong> / Perfil revisi&#243;n {calculation.profile
      .revision}
  </p>
  <p>{deadlineRuleLabel(result.rule)}</p>
  <div class="case-comparison">
    <h4>Inicio utilizado</h4>
    <p>{deadlineRequirementLabel(result.requirement)}</p>
    {#if result.trigger_outcome.kind === 'extracted'}<p>
        {factTimeLabel(result.trigger_outcome.at)}
      </p>
    {:else}<p>{deadlineBlockLabel(result.trigger_outcome.block)}</p>{/if}
  </div>
  {#if result.arithmetic}{@const outcome = result.arithmetic.outcome}
    <p>Inicio de la aritm&#233;tica: {factTimeLabel(result.arithmetic.anchor)}</p>
    {#if outcome.kind === 'civil_candidate'}<p>Fecha candidata: <strong>{outcome.date}</strong>.</p>
      <p class="hint">
        La fecha candidata conserva el c&#243;mputo civil; el vencimiento requiere la pol&#237;tica
        de finalizaci&#243;n del perfil.
      </p>
    {:else if outcome.kind === 'instant_candidate'}<p>
        Instante candidato: {deadlineInstantLabel(outcome.instant)}
      </p>
    {:else}<p>{deadlineBlockLabel(outcome.block)}</p>{/if}
  {/if}
  {#if result.blocks.length}<div class="notice warning" role="status">
      <h4>Motivos pendientes</h4>
      <ol>
        {#each result.blocks as block}<li>{deadlineBlockLabel(block)}</li>{/each}
      </ol>
    </div>{/if}
  {#if result.arithmetic?.trace.length}<details bind:open={traceOpen}>
      <summary>Ver pasos del c&#243;mputo ({result.arithmetic.trace.length})</summary>
      {#if traceOpen}<DeadlineTrace trace={result.arithmetic.trace} />{/if}
    </details>{/if}
  {#if !compact}<p class="hint">
      Resultado conservado al registrar esta revisi&#243;n, junto con el perfil y las fuentes
      utilizadas.
    </p>{/if}
</section>
