<script>
  import OwnerCertificates from './OwnerCertificates.svelte';
  import SigningCertificates from './SigningCertificates.svelte';
  export let api, user;
</script>

{#if user.role === 'owner'}<OwnerCertificates {api} {user} />
{:else if user.role === 'litigator'}<SigningCertificates {api} {user} />{/if}
