<script>
  import { caseFilters } from '../lib/case-administration.mjs';
  export let busy = false,
    onapply;
  let values = { title: '', nuc: '', judicial_case_number: '', status: 'active', profile: 'all' },
    error = '';
  function submit(event) {
    event.preventDefault();
    try {
      const query = caseFilters(values);
      error = '';
      onapply(query);
    } catch (failure) {
      error = failure.message;
    }
  }
</script>

<form class="case-filters" aria-busy={busy} onsubmit={submit}>
  <div class="case-field-grid">
    <label>Buscar por t&#237;tulo<input bind:value={values.title} /></label>
    <label>NUC exacto<input bind:value={values.nuc} /></label>
    <label>Carpeta judicial exacta<input bind:value={values.judicial_case_number} /></label>
    <label
      >Estado administrativo<select bind:value={values.status}
        ><option value="active">Activos</option><option value="closed">Cerrados</option><option
          value="all">Todos</option
        ></select
      ></label
    >
    <label
      >Ficha penal<select bind:value={values.profile}
        ><option value="all">Todas</option><option value="complete">Completas</option><option
          value="pending">Pendientes</option
        ></select
      ></label
    >
    <div class="case-filter-action">
      <button class="secondary">Buscar expedientes</button>
    </div>
  </div>
  <p class="hint">
    Busca por una parte del t&#237;tulo. El NUC y la carpeta deben coincidir por completo. Se
    distinguen may&#250;sculas y acentos.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</form>
