<script>
  // The requests in which the account is a party: "Mis contratos" for a
  // Client and "Por firmar" for a firm signer.
  // Rationale: docs/adr/0086-bilateral-contract-signing.md.
  import { onDestroy, onMount } from 'svelte';
  import { expiryText, requestStatusView } from '../lib/contract-presentation.mjs';
  import ContractList from './ContractList.svelte';
  import ContractSigningAction from './ContractSigningAction.svelte';
  export let api, user;
  export let onnavigate = () => {};
  const client = user.role === 'client';
  const requests = api.contracts().requests;
  let rows = [],
    selected = null,
    busy = false,
    loaded = false,
    error = '',
    alive = true;
  $: listed = rows.map((row) => {
    const status = requestStatusView(row.status);
    return {
      id: row.request_id,
      title: row.title,
      label: status.label,
      tone: status.tone,
      detail: `Versi\u00f3n ${row.version}. Vence: ${expiryText(row.expires_at_unix)}`,
    };
  });
  async function load() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const value = await requests.list();
      if (!alive) return;
      rows = value;
      loaded = true;
    } catch (failure) {
      if (alive) error = failure.message;
    } finally {
      if (alive) busy = false;
    }
  }
  onMount(load);
  onDestroy(() => (alive = false));
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">{client ? 'MI ESPACIO' : 'CONTRATOS'}</span>
    <h1>{client ? 'Mis contratos' : 'Por firmar'}</h1>
    <p>
      {client
        ? 'Revisa los contratos que el despacho te envi\u00f3 y f\u00edrmalos con la clave de tu equipo.'
        : 'Solicitudes en las que firmas por el despacho despu\u00e9s del Cliente.'}
    </p>
  </div>
</div>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
<ContractList
  title={client ? 'Contratos recibidos' : 'Solicitudes de firma'}
  rows={listed}
  {selected}
  {busy}
  {loaded}
  empty={client
    ? 'Todav\u00eda no tienes contratos por revisar.'
    : 'No tienes solicitudes de firma.'}
  onopen={(id) => (selected = id)}
  onrefresh={load}
/>
{#if selected}{#key selected}<ContractSigningAction
      {api}
      {requests}
      {user}
      requestId={selected}
      onchanged={load}
      {onnavigate}
    />{/key}{/if}
