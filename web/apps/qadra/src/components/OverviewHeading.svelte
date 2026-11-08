<script>
  import Button from './Button.svelte';
  // Overview page heading: today's date as the title and, below it, the
  // account name (never the email), its role and the scope of the counts.
  // The default slot fills the bottom of the card, for the counts of the roles
  // that have them.
  import { roles } from '../lib/documents.mjs';
  export let user;
  export let scope = '';
  export let onnavigate;
  const day = new Intl.DateTimeFormat('es-MX', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  }).format(new Date());
  const title = day.charAt(0).toLocaleUpperCase('es-MX') + day.slice(1);
  $: details = [user.name, roles[user.role], scope].filter(Boolean).join(' \u00b7 ');
</script>

<section class="overflow-hidden surface-card" aria-label="Tu cuenta y tus pendientes">
  <h1 class="sr-only">Tu mesa de trabajo</h1>
  <div class="flex flex-col gap-4 p-6 sm:flex-row sm:items-center sm:justify-between">
    <div class="min-w-0">
      <p class="text-page-title text-ink">{title}</p>
      <p class="mt-1 truncate text-body text-muted" title={details}>{details}</p>
    </div>
    <Button
      variant="secondary"
      type="button"
      class="self-start sm:self-auto"
      onclick={() => onnavigate('cases')}>Ver expedientes</Button
    >
  </div>
  <slot />
</section>
