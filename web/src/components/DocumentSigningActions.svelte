<script>
  import IndividualDocumentSigningAction from './IndividualDocumentSigningAction.svelte';
  import IndividualSigningStatus from './IndividualSigningStatus.svelte';
  export let api, user, document, onupdate, ondenied;
  export let disabled = false,
    busy = '',
    active = false;
</script>

{#if typeof api.individualSigning === 'function'}
  {#if user.role === 'litigator'}
    {#key `${user.id}:${document.case_id}:${document.id}:${document.version}:${document.digest}`}
      <IndividualDocumentSigningAction
        {api}
        {user}
        {document}
        {onupdate}
        {ondenied}
        {disabled}
        bind:busy
        bind:active
      />
    {/key}
  {/if}
  {#if document.sealed}<IndividualSigningStatus {api} {user} {document} {disabled} bind:busy />{/if}
{/if}
