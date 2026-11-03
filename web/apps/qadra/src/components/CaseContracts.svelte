<script>
  // The contracts of one case for its staff. Contracts are created from the
  // detail of a PDF document version. Rationale:
  // docs/adr/0086-bilateral-contract-signing.md.
  import { onDestroy, onMount } from 'svelte';
  import { contractStateView } from '../lib/contract-presentation.mjs';
  import ContractList from './ContractList.svelte';
  import ContractDetail from './ContractDetail.svelte';
  export let api,
    user,
    record,
    ondenied = () => {};
  const contracts = api.contracts();
  const scoped = contracts.caseContracts(record.id);
  let rows = [],
    firmSigners = [],
    selected = null,
    busy = false,
    loaded = false,
    error = '',
    alive = true;
  $: listed = rows.map((row) => {
    const state = contractStateView(row.state);
    return {
      id: row.contract_id,
      title: row.title,
      label: state.label,
      tone: state.tone,
      detail: `${row.requests.length} solicitud(es) de firma`,
    };
  });
  async function load() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const value = await scoped.list();
      if (!alive) return;
      rows = value.contracts;
      firmSigners = value.firm_signers;
      loaded = true;
    } catch (failure) {
      if (!alive) return;
      error = failure.message;
      if ([403, 404].includes(failure.status) && failure.code !== 'contract_not_found')
        ondenied(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  onMount(load);
  onDestroy(() => (alive = false));
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">EXPEDIENTE</span>
    <h1>Contratos</h1>
    <p>
      Contratos firmados por el Cliente y el despacho sobre una versi&#243;n exacta de un PDF. Para
      crear uno, abre el documento y elige Crear contrato.
    </p>
  </div>
</div>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
<ContractList
  title="Contratos del expediente"
  rows={listed}
  {selected}
  {busy}
  {loaded}
  empty="Este expediente no tiene contratos."
  onopen={(id) => (selected = id)}
  onrefresh={load}
/>
{#if selected}{#key selected}<ContractDetail
      {api}
      {scoped}
      requests={contracts.requests}
      {user}
      caseId={record.id}
      contractId={selected}
      {firmSigners}
      onchanged={load}
    />{/key}{/if}
