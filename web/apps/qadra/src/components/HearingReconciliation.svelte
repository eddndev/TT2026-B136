<script>
  import HearingValues from './HearingValues.svelte';
  import { hearingStatus } from '../lib/hearings.mjs';
  import { stageLabels } from '../lib/case-stages.mjs';
  export let mode, last, candidate, context, ready, canAccept, busy, oncheck, onrefresh, onaccept;
</script>

<div class="case-comparison hearing-reconciliation">
  {#if mode === 'uncertain'}<h3>Resultado del env&#237;o pendiente de confirmar</h3>
    <p>
      Conservamos el env&#237;o. Consulta su revisi&#243;n exacta; no se repetir&#225; la escritura.
    </p>
    <button class="secondary" disabled={busy} onclick={oncheck}
      >Consultar resultado del env&#237;o</button
    >
  {:else}<h3>Compara antes de preparar otro registro</h3>
    <p>Tu borrador se conserva. Consulta los datos actuales y revisa las diferencias.</p>
    <button class="secondary" disabled={busy} onclick={onrefresh}
      >Consultar audiencia y contexto actuales</button
    >
  {/if}
  {#if last}<details>
      <summary>Identificaci&#243;n del &#250;ltimo env&#237;o</summary>
      <p>
        Audiencia: <code>{last.command.hearing_id}</code> / Revisi&#243;n objetivo {last.result_revision}.
      </p>
      <p>Operaci&#243;n: <code>{last.command.operation_id}</code></p>
    </details>{/if}
  {#if ready}<section aria-label="Registro consultado para comparar">
      {#if candidate}<h4>
          Revisi&#243;n consultada {candidate.revision}: {hearingStatus[candidate.status]}
        </h4>
        <HearingValues
          values={candidate.values}
          participants={candidate.participants}
          support={candidate.support}
        />
      {:else}<p>No se encontr&#243; una audiencia actual con este identificador.</p>{/if}
      {#if context}<p>
          Contexto actual: {stageLabels[context.stage] || 'Sin etapa'} / Administraci&#243;n {context.case_revision}
          / Etapa {context.stage_revision || 'sin registro'}.
        </p>{/if}
      {#if canAccept}<button class="secondary" disabled={busy} onclick={onaccept}
          >Usar base consultada y revisar borrador</button
        >
      {:else}<p class="hint">
          Este resultado no permite volver a preparar la misma operaci&#243;n.
        </p>{/if}
    </section>{/if}
</div>
