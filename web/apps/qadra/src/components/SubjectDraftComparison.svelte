<script>
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  export let comparison, current, uncertain, conflict, pending, readonly;
  export let refresh, useCurrent;
</script>

{#if comparison}<section class="participant-comparison" aria-label="Datos del candidato consultado">
    <h3>Datos del candidato consultado</h3>
    {#if comparison.values}<ParticipantSubjectSummary record={comparison} />
    {:else}<ParticipantSummary record={comparison} />{/if}
    <p class="hint">
      Consultar no cambia la identidad que est&#225;s editando. Registra una decisi&#243;n distinta
      con soporte si corresponde; las identidades no se fusionan.
    </p>
  </section>{/if}
{#if conflict || uncertain}<button
    class="secondary"
    disabled={pending}
    onclick={() => refresh(false)}
    >{uncertain
      ? 'Consultar revisi\u00f3n enviada de identidad'
      : 'Consultar identidad actual para comparar'}</button
  >{/if}
{#if uncertain}<button class="secondary" disabled={pending} onclick={() => refresh(true)}
    >Consultar identidad actual antes de decidir</button
  >{/if}
{#if current}<section
    class="participant-comparison"
    aria-label="Revisi&#243;n de identidad consultada"
  >
    <h3>Revisi&#243;n de identidad consultada</h3>
    <ParticipantSubjectSummary record={current} />
    <p>
      Esta consulta no atribuye el registro a tu env&#237;o. Compara los datos con tu formulario
      conservado antes de otra edici&#243;n.
    </p>
    <button class="secondary" disabled={pending || readonly} onclick={useCurrent}
      >Usar esta base y conservar el formulario de identidad</button
    >
  </section>{/if}
