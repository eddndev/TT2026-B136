<script>
  import HeroIcon from './HeroIcon.svelte';
  import ShellSearch from './ShellSearch.svelte';
  import ShellNotifications from './ShellNotifications.svelte';
  import AccountMenu from './AccountMenu.svelte';
  import { canAlerts } from '../lib/alerts-presentation.mjs';
  import { staffCase } from '../lib/case-administration.mjs';
  export let api;
  export let user;
  export let view;
  export let onmenu;
  export let onnavigate;
  export let onopencase;
  export let onlogout;
</script>

<header
  class="sticky top-0 z-40 flex h-16 shrink-0 items-center gap-x-4 border-b border-line bg-white px-4 shadow-xs sm:gap-x-6 sm:px-6 lg:px-8"
>
  <button
    type="button"
    class="-m-2.5 p-2.5 text-slate-700 hover:text-ink lg:hidden"
    aria-label="Abrir men&#250;"
    onclick={() => onmenu()}><HeroIcon name="menu" class="size-6" /></button
  >
  <div aria-hidden="true" class="h-6 w-px bg-line lg:hidden"></div>
  <div class="flex flex-1 gap-x-4 self-stretch lg:gap-x-6">
    {#if staffCase(user.role)}
      <ShellSearch {api} {onopencase} onallcases={() => onnavigate('cases')} />
    {:else}
      <div class="flex-1"></div>
    {/if}
    <div class="flex items-center gap-x-4 lg:gap-x-6">
      {#if canAlerts(user.role)}<ShellNotifications {api} {user} {view} {onnavigate} />{/if}
      <div aria-hidden="true" class="hidden lg:block lg:h-6 lg:w-px lg:bg-line"></div>
      <AccountMenu {user} {onlogout} />
    </div>
  </div>
</header>
