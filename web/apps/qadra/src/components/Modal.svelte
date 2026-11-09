<script>
  // Shared modal. It stays a native <dialog> opened with showModal(), so the
  // browser provides the top layer, focus containment and Escape, and the
  // session pause logic (src/lib/session-dialogs.mjs) can find and reopen it.
  // Callers bind the element to call showModal() and close() themselves.
  // Centered from sm up; on phones it rests at the bottom of the screen.
  // Only opening is animated, through starting styles. Closing is immediate:
  // transitioning display or overlay would keep a closed dialog rendered and
  // in the top layer, so reopening a modal would briefly show two of them.
  export let element = null;
  // Panel width from sm up.
  export let size = 'md';
  let className = '';
  export { className as class };
  const widths = { sm: 'sm:max-w-sm', md: 'sm:max-w-xl', lg: 'sm:max-w-3xl' };
</script>

<dialog
  bind:this={element}
  class={`fixed inset-0 mx-auto mt-auto mb-4 h-fit max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] max-w-none overflow-y-auto rounded-lg bg-surface px-4 pt-5 pb-4 text-left text-ink shadow-xl transition-[opacity,translate,scale] duration-300 ease-out backdrop:bg-muted/75 motion-reduce:transition-none starting:open:translate-y-4 starting:open:opacity-0 sm:my-auto sm:p-6 sm:starting:open:translate-y-0 sm:starting:open:scale-95 ${widths[size]} ${className}`.trim()}
  {...$$restProps}
>
  <slot />
</dialog>
