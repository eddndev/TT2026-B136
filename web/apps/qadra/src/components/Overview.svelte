<script>
  // Landing view of office accounts. Owner and Litigator get the Dashboard;
  // the other roles get the heading, the shortcuts, the current case and,
  // until a case is selected, the steps a document follows.
  import Button from './Button.svelte';
  import Dashboard from './Dashboard.svelte';
  import OverviewHeading from './OverviewHeading.svelte';
  import OverviewLinks from './OverviewLinks.svelte';
  export let user;
  export let api;
  export let selectedCase = null;
  export let onnavigate;
  export let ondocument;
  const steps = [
    { title: 'Carga', detail: 'El archivo se guarda cifrado' },
    { title: 'Sella', detail: 'Genera la firma y el sello de tiempo' },
    { title: 'Verifica y descarga', detail: 'Revisa y conserva tu evidencia' },
  ];
</script>

{#if ['owner', 'litigator'].includes(user.role)}
  <Dashboard {api} {user} {onnavigate} {ondocument} />
{:else}
  <div class="grid gap-8">
    <OverviewHeading {user} {onnavigate} />
    <div class="grid gap-8 xl:grid-cols-3">
      <section class="surface-card p-6 xl:col-span-2" aria-labelledby="overview-case-title">
        <h2 id="overview-case-title" class="text-card-title text-ink">Tu expediente actual</h2>
        {#if selectedCase}
          <p class="mt-4 text-label text-ink">{selectedCase.title}</p>
          <p class="mt-1 font-mono text-body text-ink-secondary">{selectedCase.reference}</p>
          <Button
            variant="secondary"
            type="button"
            class="mt-4"
            onclick={() => onnavigate('documents')}>Abrir expediente</Button
          >
        {:else}
          <p class="mt-4 text-body text-muted">
            Selecciona un expediente para consultar sus datos y documentos
          </p>
          <Button variant="secondary" type="button" class="mt-4" onclick={() => onnavigate('cases')}
            >Seleccionar expediente</Button
          >
        {/if}
        <p class="mt-6 border-t border-line pt-4 text-caption text-muted">
          Los expedientes y documentos permanecen guardados al cerrar sesi&#243;n
        </p>
      </section>
      <div class="grid grid-cols-1 content-start gap-8">
        <OverviewLinks {user} {onnavigate} {ondocument} />
        {#if !selectedCase}
          <section class="surface-card p-6" aria-labelledby="overview-steps-title">
            <h2 id="overview-steps-title" class="text-card-title text-ink">Tu siguiente paso</h2>
            <p class="mt-1 text-body text-muted">
              Cada archivo sigue un recorrido sencillo dentro de su expediente
            </p>
            <ol class="mt-4 grid gap-4">
              {#each steps as step, index (step.title)}
                <li class="flex gap-x-3">
                  <span
                    class="flex size-6 shrink-0 items-center justify-center rounded-full text-caption text-ink-secondary tabular-nums ring-1 ring-line"
                    >{index + 1}</span
                  >
                  <span class="min-w-0">
                    <span class="block text-label text-ink">{step.title}</span>
                    <span class="block text-body text-muted">{step.detail}</span>
                  </span>
                </li>
              {/each}
            </ol>
          </section>
        {/if}
      </div>
    </div>
  </div>
{/if}
