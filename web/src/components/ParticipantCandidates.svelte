<script>
  import ParticipantSupport from './ParticipantSupport.svelte';
  import { candidateKey } from '../lib/typed-participant-values.mjs';
  export let result,
    decisions = {},
    reason = '',
    docs,
    caseId,
    ondenied,
    onchoose,
    disabled = false,
    pending = false;
  export let choiceLabel = 'Consultar y usar este candidato';
  let pendingRows = {};
  const signals = {
    name: 'Nombre',
    declared_identifier: 'Identificador declarado',
    certificate: 'Certificado',
    documentary_evidence: 'Soporte documental',
  };
  $: pending = Object.values(pendingRows).some(Boolean);
</script>

<section class="participant-comparison" aria-label="Revisi&#243;n de identidad">
  <h3>Revisi&#243;n de identidad</h3>
  <p class="hint">
    Revisa cada coincidencia. Un nombre compartido no permite fusionar personas. La ausencia de
    coincidencias no acredita una identidad.
  </p>
  {#if !result.candidates.length}<p>No hay candidatos en esta consulta.</p>{/if}
  {#each result.candidates as row (candidateKey(row.reference))}
    {@const key = candidateKey(row.reference)}
    <section class="participant-revision" aria-label={`Candidato ${row.display_name}`}>
      <h4>{row.display_name}</h4>
      <p>
        {row.reference.kind === 'subject' ? 'Identidad registrada' : 'Ficha pendiente de tipificar'} /
        revisi&#243;n {row.reference.revision}
      </p>
      <p class="hint">Coincidencias: {row.signals.map((signal) => signals[signal]).join(', ')}</p>
      <div class="action-row">
        <button
          type="button"
          class="secondary"
          disabled={disabled || pending}
          onclick={() => onchoose(row)}>{choiceLabel}</button
        >
        <button
          type="button"
          class="text-button"
          disabled={disabled || pending}
          onclick={() => (decisions = { ...decisions, [key]: { reason: '', support: null } })}
          >Declarar persona distinta</button
        >
      </div>
      {#if decisions[key]}<label
          >Motivo de candidato distinto<textarea
            bind:value={decisions[key].reason}
            disabled={disabled || pending}></textarea></label
        >
        <ParticipantSupport
          api={docs}
          {caseId}
          label="Soporte de comparaci&#243;n"
          bind:value={decisions[key].support}
          {ondenied}
          disabled={disabled ||
            Object.entries(pendingRows).some(([id, value]) => id !== key && value)}
          bind:pending={pendingRows[key]}
        />{/if}
    </section>
  {/each}
  <label
    >Motivo de selecci&#243;n de identidad<textarea
      bind:value={reason}
      disabled={disabled || pending}></textarea></label
  >
</section>
