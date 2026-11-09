import { tick } from 'svelte';

// A form that takes the place of a record card receives focus on its heading,
// which also scrolls it into view.
export async function focusForm(selector) {
  await tick();
  const heading = document.querySelector(`${selector} h2`);
  if (!heading) return;
  heading.tabIndex = -1;
  heading.focus();
}

// When the record card comes back, focus returns to its first enabled action.
// The card heading is the fallback while its buttons are still locked.
export async function focusCard(selector) {
  await tick();
  await tick();
  const card = document.querySelector(selector);
  const target = card?.querySelector(':scope > .action-row button:not(:disabled)');
  if (target) return target.focus();
  const heading = card?.querySelector('h2');
  if (!heading) return;
  heading.tabIndex = -1;
  heading.focus();
}
