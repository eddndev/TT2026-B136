<script>
  import { profileKinds } from '../lib/typed-participant-fields.mjs';
  let kind = '',
    profile = 'all';
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
      onapply(participantFilters({ name, procedural_role: role, status, kind, profile }));
    } catch (failure) {
      error = failure.message;
      (failure.field === 'display_name' ? nameInput : roleInput)?.focus();
    }
  }
  function clear() {
    name = '';
    role = '';
    status = 'active';
    kind = '';
    profile = 'all';
    error = '';
    onapply({ status });
  }
</script>

<form class="participant-filters" onsubmit={submit}>
  <div class="participant-filter-fields">
    <label>Buscar por nombre<input bind:this={nameInput} bind:value={name} /></label>
    <label>Rol manual exacto<input bind:this={roleInput} bind:value={role} /></label>
    <label
      >Estado del directorio<select aria-label="Estado del directorio" bind:value={status}
        ><option value="active">Activos</option><option value="archived">Archivados</option><option
          value="all">Todos</option
        ></select
      ></label
    >
  </div>
  <div class="participant-filter-fields">
    <label
      >Tipo tipificado<select aria-label="Tipo tipificado" bind:value={kind}
        ><option value="">Todos los tipos</option>{#each profileKinds as item}<option
            value={item.key}>{item.label}</option
          >{/each}</select
      ></label
    ><label
      >Perfil del participante<select aria-label="Perfil del participante" bind:value={profile}
        ><option value="all">Todos</option><option value="typed">Tipificado</option><option
          value="manual">Pendiente de tipificar</option
        ></select
      ></label
    >
  </div>
  <p class="hint">
    Escribe una parte del nombre. El rol manual debe coincidir completo y solo busca en fichas
    pendientes de tipificar. Se distinguen may&#250;sculas y acentos.
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
