<script>
  import OwnerCertificates from './OwnerCertificates.svelte';
  import SigningCertificates from './SigningCertificates.svelte';
  export let api, user, view;
</script>

{#if view === 'owner-certificates' && user.role === 'owner'}<OwnerCertificates {api} {user} />
{:else if view === 'signing-certificates' && ['owner', 'litigator', 'client'].includes(user.role)}<SigningCertificates
    {api}
    {user}
  />{/if}
