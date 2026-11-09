<script>
  // "Mi firma digital": one page for the account's two certificates. They stay
  // separate tabs because each one has its own key and CA purpose: document
  // signing for every signer, and certificate access for the Owner only.
  import OwnerCertificates from './OwnerCertificates.svelte';
  import SigningCertificates from './SigningCertificates.svelte';
  import SectionTabs from './SectionTabs.svelte';
  export let api, user, view, onnavigate;
  $: owner = user.role === 'owner';
  const tabs = [
    { id: 'signing-certificates', label: 'Firma de documentos', icon: 'fingerprint' },
    { id: 'owner-certificates', label: 'Acceso con certificado', icon: 'key' },
  ];
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">Mi cuenta</span>
    <h1>Mi firma digital</h1>
    <p>Tus certificados y las claves que guardas en tu equipo.</p>
  </div>
</div>
{#if owner}
  <SectionTabs
    items={tabs}
    current={view}
    label="Secciones de mi firma digital"
    onselect={onnavigate}
  />
{/if}
<p class="notice mb-4">
  Usa certificados de la CA interna del despacho; no sustituyen a la e.firma del SAT.
</p>
{#if view === 'owner-certificates' && owner}<OwnerCertificates {api} {user} />
{:else if view === 'signing-certificates' && ['owner', 'litigator', 'client'].includes(user.role)}<SigningCertificates
    {api}
    {user}
  />{/if}
