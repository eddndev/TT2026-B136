<script>
  import Button from './Button.svelte';
  import FilterBar from './FilterBar.svelte';
  import FilterField from './FilterField.svelte';
  import MoreFilters from './MoreFilters.svelte';
  import { caseFilters } from '../lib/case-administration.mjs';
  export let busy = false,
    onapply;
  let values = { title: '', nuc: '', judicial_case_number: '', status: 'active', profile: 'all' },
    error = '';
  $: exact = [values.nuc, values.judicial_case_number].filter((value) => value.trim()).length;
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

<form class="case-search" aria-busy={busy} onsubmit={submit}>
  <FilterBar>
    <FilterField size="wide">Buscar por t&#237;tulo<input bind:value={values.title} /></FilterField>
    <FilterField
      >Estado administrativo<select bind:value={values.status}
        ><option value="active">Activos</option><option value="closed">Cerrados</option><option
          value="all">Todos</option
        ></select
      ></FilterField
    >
    <FilterField
      >Ficha penal<select bind:value={values.profile}
        ><option value="all">Todas</option><option value="complete">Completas</option><option
          value="pending">Pendientes</option
        ></select
      ></FilterField
    >
    <Button variant="secondary">Buscar expedientes</Button>
  </FilterBar>
  <MoreFilters count={exact}>
    <FilterBar>
      <FilterField>NUC exacto<input bind:value={values.nuc} /></FilterField>
      <FilterField
        >Carpeta judicial exacta<input bind:value={values.judicial_case_number} /></FilterField
      >
    </FilterBar>
    <p class="filter-note">
      El t&#237;tulo admite una parte. El NUC y la carpeta deben coincidir por completo. Se
      distinguen may&#250;sculas y acentos.
    </p>
  </MoreFilters>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</form>
