<script>
  export let dependency,
    value = '',
    present = false,
    disabled = false;
  const labels = {
    profile: 'Cuando cambie el perfil',
    source: 'Cuando cambie la fuente',
    calendar: 'Cuando cambie el calendario',
  };
</script>

{#if present}
  <label>
    {labels[dependency]}
    <select bind:value {disabled} required>
      <option value="">Selecciona c&#243;mo dar seguimiento</option>
      <option value="fixed">Conservar esta revisi&#243;n</option>
      <option value="follow"
        >{dependency === 'calendar'
          ? 'Seguir cambios del calendario'
          : 'Seguir cambios y solicitar revisi\u00f3n'}</option
      >
    </select>
  </label>
  {#if value === 'fixed'}
    <p class="hint">
      Los cambios ordinarios conservan la selecci&#243;n. Un retiro puede requerir revisi&#243;n.
    </p>
  {:else if value === 'follow'}
    <p class="hint">
      {dependency === 'calendar'
        ? 'Un calendario publicado puede actualizar el c\u00e1lculo si el seguimiento sigue aceptado.'
        : 'Los cambios requieren revisar la aplicabilidad y conservan tus declaraciones.'}
    </p>
  {/if}
{:else}
  <p class="hint">Sin dependencia seleccionada.</p>
{/if}
