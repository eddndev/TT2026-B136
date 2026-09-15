<script>
  import { participantFilters } from '../lib/participants.mjs';
  export let onapply;
  export let busy;
  let name = '',
    role = '',
    status = 'active',
    error = '';
  let nameInput, roleInput;
  function submit(event) {
    event.preventDefault();
    error = '';
    try {
      onapply(participantFilters({ name, procedural_role: role, status }));
    } catch (failure) {
      error = failure.message;
      (failure.field === 'display_name' ? nameInput : roleInput)?.focus();
    }
  }
  function clear() {
    name = '';
    role = '';
    status = 'active';
    error = '';
    onapply({ status });
  }
</script>

<form class="participant-filters" onsubmit={submit}>
  <div class="participant-filter-fields">
    <label>Buscar por nombre<input bind:this={nameInput} bind:value={name} /></label>
    <label>Rol exacto<input bind:this={roleInput} bind:value={role} /></label>
    <label
      >Estado del directorio<select aria-label="Estado del directorio" bind:value={status}
        ><option value="active">Activos</option><option value="archived">Archivados</option><option
          value="all">Todos</option
        ></select
      ></label
    >
  </div>
  <p class="hint">
    Escribe una parte del nombre; el rol debe coincidir completo. Se distinguen may&#250;sculas y
    acentos.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <div class="action-row">
    <button class="secondary" disabled={busy}>Aplicar filtros</button><button
      type="button"
      class="text-button"
      onclick={clear}>Limpiar filtros</button
    >
  </div>
</form>
