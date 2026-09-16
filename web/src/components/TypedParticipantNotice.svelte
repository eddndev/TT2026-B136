<script>
  import ParticipantSummary from './ParticipantSummary.svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  import ParticipantConflictDirectory from './ParticipantConflictDirectory.svelte';
  export let api,
    kind,
    ondenied,
    directoryDisabled = false,
    directoryBusy = false;
  let showDirectory = false;
  export let prepared, error, conflict, current, uncertain, checkedAbsent, pending, closed;
  export let onrefresh, onusecurrent, onreconcile, onresend;
</script>

{#if prepared && !prepared.declaration}<p class="notice">
    Registro preparado sin firma personal
  </p>{/if}
{#if prepared}<p class="hint">
    Preparaci&#243;n para la revisi&#243;n {prepared.submission_revision}. A&#250;n no se ha
    guardado la ficha.
  </p>{/if}
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
{#if conflict}<button class="secondary" disabled={pending} onclick={onrefresh}
    >Consultar datos actuales</button
  >{/if}
{#if current?.record || current?.identity}<section
    class="participant-comparison"
    aria-label="Ficha actual consultada"
  >
    <h3>{current.exact ? 'Revisi\u00f3n exacta consultada' : 'Ficha actual consultada'}</h3>
    {#if current.record}<ParticipantSummary
        record={current.record}
      />{/if}{#if current.identity}<ParticipantSubjectSummary record={current.identity} />{/if}
    <p>
      Tu formulario se conserva. Al usar esta base deber&#225;s revisar los candidatos y preparar
      otra declaraci&#243;n.
    </p>
    {#if !current.exact}<button
        class="secondary"
        disabled={pending || closed}
        onclick={onusecurrent}>Usar esta base y conservar mi formulario</button
      >{/if}
  </section>{/if}
{#if uncertain}<button class="secondary" disabled={pending} onclick={onreconcile}
    >Consultar resultado del env&#237;o</button
  >{#if checkedAbsent}<button class="secondary" disabled={pending || closed} onclick={onresend}
      >Reenviar el mismo registro</button
    >{/if}{/if}
{#if error && !pending}<button class="text-button" onclick={() => (showDirectory = !showDirectory)}
    >Consultar directorio completo</button
  >{/if}

{#if showDirectory}<ParticipantConflictDirectory
    {api}
    {kind}
    {ondenied}
    disabled={directoryDisabled}
    bind:busy={directoryBusy}
  />{/if}
