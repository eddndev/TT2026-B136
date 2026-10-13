<script>
  import { roles } from '../lib/documents.mjs';
  import { memberQuery } from '../lib/members-query.mjs';
  export let caseScope = false,
    disabled = false,
    busy = false,
    onapply;
  let prefix = '',
    role = '',
    mode = caseScope ? 'assigned' : 'active',
    error = '';
  function submit(event) {
    event.preventDefault();
    try {
      const query = memberQuery(
        {
          limit: 20,
          [caseScope ? 'selection' : 'status']: mode,
          ...(role ? { role } : {}),
          email_prefix: prefix,
        },
        caseScope,
      );
      error = '';
      onapply(query);
    } catch (failure) {
      error = failure.message;
    }
  }
</script>

<form class="case-filters" aria-busy={busy} onsubmit={submit}>
  <div class="case-field-grid">
    <label>Buscar por correo<input {disabled} autocomplete="off" bind:value={prefix} /></label>
    <label
      >Rol de la cuenta<select {disabled} bind:value={role}>
        <option value="">Todos los roles</option>
        {#each Object.entries(roles) as [value, label]}<option {value}>{label}</option>{/each}
      </select></label
    >
    <label
      >{caseScope ? 'Selecci\u00f3n de cuentas' : 'Estado de la cuenta'}
      <select {disabled} bind:value={mode}>
        {#if caseScope}<option value="assigned">Cuentas asignadas</option><option value="available"
            >Cuentas disponibles</option
          >
        {:else}<option value="active">Activas</option><option value="inactive">Inactivas</option
          ><option value="all">Todas</option>{/if}
      </select>
    </label>
    <div class="case-filter-action">
      <button class="secondary" {disabled}>
        {caseScope ? 'Buscar asignaciones' : 'Buscar cuentas'}
      </button>
    </div>
  </div>
  <p class="hint">
    El correo se busca por prefijo. Los resultados conservan el orden por identificador.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</form>
