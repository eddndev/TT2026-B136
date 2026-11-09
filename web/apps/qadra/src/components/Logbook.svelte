<script>
  // Owner logbook: the hash-chained movements and the integrity incidents they
  // refer to, as two tabs of one page. Each tab keeps its own view id.
  import AuditChainCheck from './AuditChainCheck.svelte';
  import AuditEvents from './AuditEvents.svelte';
  import DocumentIntegrityIncidents from './DocumentIntegrityIncidents.svelte';
  import SectionTabs from './SectionTabs.svelte';
  export let api, user, tab, onnavigate, onopen, onknown;
  const tabs = [
    { id: 'audit', label: 'Movimientos', icon: 'audit' },
    { id: 'integrity-incidents', label: 'Incidentes de integridad', icon: 'warning' },
  ];
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">Administraci&oacute;n</span>
    <h1>Bit&#225;cora</h1>
    <p>Movimientos del despacho e incidentes de integridad documental.</p>
  </div>
</div>
<SectionTabs
  items={tabs}
  current={tab}
  label="Secciones de la bit&#225;cora"
  onselect={onnavigate}
/>
{#if tab === 'audit'}
  <div class="space-y-6">
    <AuditChainCheck {api} />
    <AuditEvents {api} {user} />
  </div>
{:else}
  <DocumentIntegrityIncidents {api} {onopen} {onknown} />
{/if}
