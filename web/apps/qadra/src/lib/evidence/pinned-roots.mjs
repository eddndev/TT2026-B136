// The validated pinned roots of this build. The alias points at the release
// file or at the file named by QADRA_PINNED_ROOTS; see astro.config.mjs.
import roots from '@qadra/pinned-roots';
import { parsePinnedRoots } from './pinned-roots-schema.mjs';

export const pinnedRoots = parsePinnedRoots(roots);
