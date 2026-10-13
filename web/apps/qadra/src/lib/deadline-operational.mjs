import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { deadlineInstant } from './deadline-time.mjs';

// Currentness is supplied by an authorized server read, never inferred locally.
export function deadlineOperational(raw, context, exact = false) {
  object(raw, ['freshness', 'checked_at', 'changed_dependencies', 'due_at']);
  if (
    !['current', 'changed', 'not_checked'].includes(raw.freshness) ||
    !Array.isArray(raw.changed_dependencies)
  )
    invalid();
  if ((exact || context.receiptKind === 'v1') && raw.freshness !== 'not_checked') invalid();
  if (raw.freshness === 'not_checked') {
    if (raw.checked_at !== null || raw.changed_dependencies.length || raw.due_at !== null)
      invalid();
    return raw;
  }
  deadlineInstant(raw.checked_at);
  if (raw.freshness === 'current') {
    if (raw.changed_dependencies.length) invalid();
  } else {
    if (!raw.changed_dependencies.length || raw.changed_dependencies.length > 3) invalid();
    let previous = -1;
    for (const dependency of raw.changed_dependencies) {
      const index = ['profile', 'source', 'calendar'].indexOf(dependency);
      if (index <= previous) invalid();
      previous = index;
    }
    if (raw.due_at !== null) invalid();
  }
  if (raw.due_at !== null) {
    deadlineInstant(raw.due_at);
    if (
      raw.freshness !== 'current' ||
      context.receiptKind !== 'v2' ||
      context.status !== 'active' ||
      context.reviewState !== 'accepted' ||
      context.dueAt === null ||
      !same(raw.due_at, context.dueAt)
    )
      invalid();
  }
  return raw;
}
