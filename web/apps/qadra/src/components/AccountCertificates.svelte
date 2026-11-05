<script>
  // "Mi firma digital": one page for the account's two certificates. They stay
  // separate tabs because each one has its own key and CA purpose: document
  // signing for every signer, and certificate access for the Owner only.
  import OwnerCertificates from './OwnerCertificates.svelte';
  import SigningCertificates from './SigningCertificates.svelte';
  export let api, user, view, onnavigate;
  $: owner = user.role === 'owner';
  const tabs = [
    { id: 'signing-certificates', label: 'Firma de documentos' },
    { id: 'owner-certificates', label: 'Acceso con certificado' },
  ];
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">MI CUENTA</span>
    <h1>Mi firma digital</h1>
    <p>Tus certificados y las claves que guardas en tu equipo.</p>
  </div>
</div>
{#if owner}
  <div class="mb-6 flex gap-x-6 border-b border-line" aria-label="Secciones de mi firma digital">
    {#each tabs as item}
      <button
        class={`-mb-px border-b-2 px-1 pb-3 text-sm font-semibold ${
          view === item.id
            ? 'border-brand-800 text-brand-800'
            : 'border-transparent text-slate-500 hover:border-slate-300 hover:text-slate-700'
        }`}
        aria-current={view === item.id ? 'page' : undefined}
        onclick={() => onnavigate(item.id)}>{item.label}</button
      >
    {/each}
  </div>
{/if}
<p class="notice mb-4">
  Usa certificados de la CA interna del despacho; no sustituyen a la e.firma del SAT.
</p>
{#if view === 'owner-certificates' && owner}<OwnerCertificates {api} {user} />
{:else if view === 'signing-certificates' && ['owner', 'litigator', 'client'].includes(user.role)}<SigningCertificates
    {api}
    {user}
  />{/if}
