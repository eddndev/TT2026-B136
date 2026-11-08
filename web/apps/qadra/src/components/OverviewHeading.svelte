<script>
  import Button from './Button.svelte';
  // Overview page heading: who is signed in, greeted by account name (never by
  // email), and, for roles with indicators, the three counts that need attention first.
  import { roles } from '../lib/documents.mjs';
  export let user;
  export let scope = '';
  // Each stat is { key, label, value, urgent } or the whole list is null.
  export let stats = null;
  export let onnavigate;
  const today = new Intl.DateTimeFormat('es-MX', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  }).format(new Date());
  $: details = [roles[user.role], scope, today].filter(Boolean);
</script>

<section
  class="mb-8 overflow-hidden rounded-lg bg-white shadow-sm ring-1 ring-line"
  aria-label="Tu cuenta y tus pendientes"
>
  <h1 class="sr-only">Tu mesa de trabajo</h1>
  <div class="p-6 sm:flex sm:items-center sm:justify-between sm:gap-x-6">
    <div class="min-w-0 text-center sm:text-left">
      <p class="text-sm font-medium text-muted">Hola de nuevo,</p>
      {#if user.name}<p class="truncate text-xl font-bold text-ink sm:text-2xl">{user.name}</p>{/if}
      <p class="text-sm font-medium text-muted">
        {#each details as part, index}{#if index}<span class="mx-1.5">&#183;</span>{/if}<span
            class={index === details.length - 1 ? 'inline-block first-letter:uppercase' : ''}
            >{part}</span
          >{/each}
      </p>
    </div>
    <div class="mt-5 flex shrink-0 justify-center sm:mt-0">
      <Button variant="secondary" type="button" onclick={() => onnavigate('cases')}
        >Ver expedientes</Button
      >
    </div>
  </div>
  {#if stats}
    <div
      class="grid grid-cols-1 divide-y divide-line border-t border-line bg-slate-50 sm:grid-cols-3 sm:divide-x sm:divide-y-0"
    >
      {#each stats as item (item.key)}
        <div
          class="px-6 py-5 text-center text-sm font-medium"
          class:dashboard-urgent={item.urgent && item.value > 0}
          data-metric={item.key}
        >
          <strong class={item.urgent && item.value > 0 ? 'text-red-700' : 'text-ink'}
            >{item.value}</strong
          >
          <span class="text-slate-600">{item.label}</span>
        </div>
      {/each}
    </div>
  {/if}
</section>
