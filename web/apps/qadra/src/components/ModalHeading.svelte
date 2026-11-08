<script>
  // Title row of a modal: an optional eyebrow, the title the dialog is
  // labelled by (pass its id to the Modal's aria-labelledby) and, in the slot,
  // the control that closes the modal.
  export let title;
  export let id = undefined;
  export let eyebrow = '';
</script>

<div class="mb-4 flex items-start justify-between gap-x-4">
  <div class="min-w-0">
    {#if eyebrow}<p class="text-caption text-muted">{eyebrow}</p>{/if}
    <h2 {id} class="text-section-title text-ink">{title}</h2>
  </div>
  <slot />
</div>
