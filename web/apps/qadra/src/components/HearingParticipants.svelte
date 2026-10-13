<script>
  import HearingReference from './HearingReference.svelte';
  import { profileLabel } from '../lib/typed-participant-fields.mjs';
  export let rows = [],
    onremove = null,
    disabled = false;
</script>

<div class="hearing-participants">
  {#if !rows.length}<p class="hint">Sin selecci&#243;n de participantes registrada.</p>{/if}
  {#each rows as row (`${row.id}:${row.revision}`)}
    <div class="hearing-participant">
      <div>
        <strong>{row.display_name}</strong>
        <p>
          {row.profile === 'typed' || row.kind
            ? profileLabel(row.procedural_role)
            : row.procedural_role} / Ficha, revisi&#243;n {row.revision}
        </p>
        {#if row.subject}<p class="hint">
            Identidad vinculada, revisi&#243;n {row.subject.revision}
          </p>{/if}
        {#if row.retained}<p class="hint">Referencia conservada del registro anterior.</p>{/if}
        <HearingReference {row} />
        <small
          >{row.directory_status === 'archived'
            ? 'Archivado en esta revisi\u00f3n'
            : 'Activo en esta revisi\u00f3n'}</small
        >
      </div>
      {#if onremove}<button
          type="button"
          class="text-button"
          {disabled}
          onclick={() => onremove(row.id)}
          aria-label={`Quitar participante ${row.display_name}`}>Quitar</button
        >{/if}
    </div>
  {/each}
  <p class="hint">
    Las fichas vinculadas conservan sus revisiones. No acreditan asistencia a la audiencia.
  </p>
</div>
