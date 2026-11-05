<script>
  // Owner logbook: the hash-chained movements and the integrity incidents they
  // refer to, as two tabs of one page. Each tab keeps its own view id.
  import AuditChainCheck from './AuditChainCheck.svelte';
  import AuditEvents from './AuditEvents.svelte';
  import DocumentIntegrityIncidents from './DocumentIntegrityIncidents.svelte';
  export let api, user, tab, onnavigate, onopen, onknown;
  const tabs = [
    { id: 'audit', label: 'Movimientos' },
    { id: 'integrity-incidents', label: 'Incidentes de integridad' },
  ];
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">ADMINISTRACI&#211;N</span>
    <h1>Bit&#225;cora</h1>
    <p>Movimientos del despacho e incidentes de integridad documental.</p>
  </div>
</div>
<div class="mb-6 flex gap-x-6 border-b border-line" aria-label="Secciones de la bit&#225;cora">
  {#each tabs as item}
    <button
      class={`-mb-px border-b-2 px-1 pb-3 text-sm font-semibold ${
        tab === item.id
          ? 'border-brand-800 text-brand-800'
          : 'border-transparent text-slate-500 hover:border-slate-300 hover:text-slate-700'
      }`}
      aria-current={tab === item.id ? 'page' : undefined}
      onclick={() => onnavigate(item.id)}>{item.label}</button
    >
  {/each}
</div>
{#if tab === 'audit'}
  <div class="space-y-6">
    <AuditChainCheck {api} />
    <AuditEvents {api} {user} />
  </div>
{:else}
  <DocumentIntegrityIncidents {api} {onopen} {onknown} />
{/if}
