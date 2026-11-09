<script>
  import HeroIcon from './HeroIcon.svelte';
  import { dismissable } from '../lib/dismissable.mjs';
  import { roles } from '../lib/documents.mjs';
  import { accountInitials } from '../lib/account-name.mjs';
  export let user;
  export let onlogout;
  export let busy = false;
  let open = false,
    trigger;
  // The account name leads; the email stays as the secondary line.
  $: initials = accountInitials(user);
  $: label = user.name || user.email;
  function close(event) {
    open = false;
    if (event?.key === 'Escape') trigger.focus();
  }
</script>

<div class="relative" use:dismissable={close}>
  <button
    bind:this={trigger}
    type="button"
    class="relative flex items-center"
    aria-label="Tu cuenta"
    aria-expanded={open}
    aria-controls="shell-account"
    onclick={() => (open = !open)}
  >
    <span class="absolute -inset-1.5"></span>
    <span
      class="flex size-8 items-center justify-center rounded-full bg-navy text-caption font-medium text-white"
      aria-hidden="true">{initials}</span
    >
    <span class="hidden lg:flex lg:items-center">
      <span aria-hidden="true" class="ml-4 max-w-56 truncate text-label leading-6 text-ink"
        >{label}</span
      >
      <HeroIcon name="chevron" class="ml-2 size-4 text-muted" />
    </span>
  </button>
  {#if open}
    <div
      id="shell-account"
      class="absolute right-0 z-50 mt-2.5 w-64 origin-top-right rounded-md bg-surface py-2 shadow-lg outline-1 outline-ink/5"
    >
      <div class="border-b border-line px-3 pb-3">
        <p class="truncate text-label text-ink" title={label}>{label}</p>
        {#if user.name}<p class="truncate text-caption text-muted" title={user.email}>
            {user.email}
          </p>{/if}
        <p class="text-caption text-muted">{roles[user.role] || user.role}</p>
      </div>
      <button
        type="button"
        class="mt-1 flex w-full items-center gap-x-2 px-3 py-1.5 text-left text-body leading-6 text-ink hover:bg-canvas focus:bg-canvas focus:outline-hidden disabled:opacity-60"
        disabled={busy}
        onclick={() => {
          open = false;
          onlogout();
        }}
        ><HeroIcon name="logout" class="size-5 text-muted" />{busy
          ? 'Cerrando...'
          : 'Cerrar sesi\u00f3n'}</button
      >
    </div>
  {/if}
</div>
