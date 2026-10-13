import { getContext } from 'svelte';
import { readable } from 'svelte/store';
export function caseState() {
  return getContext('case-administration') || readable({ closed: false });
}
