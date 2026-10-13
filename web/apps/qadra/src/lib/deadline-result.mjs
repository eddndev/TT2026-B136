import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { factTime } from './procedural-fact-time.mjs';
import { deadlineInstant } from './deadline-time.mjs';
import {
  deadlineRequirement,
  deadlineTriggerBlock,
  deadlineRule,
  deadlineOutcome,
  deadlineEvaluationBlock,
} from './deadline-rule.mjs';
import { deadlineTrace } from './deadline-trace.mjs';
export function deadlineResult(raw) {
  object(raw, ['requirement', 'trigger_outcome', 'rule', 'arithmetic', 'due_at', 'blocks']);
  deadlineRequirement(raw.requirement);
  const trigger = raw.trigger_outcome;
  if (trigger?.kind === 'extracted') {
    object(trigger, ['kind', 'at']);
    if (!same(factTime(trigger.at), trigger.at)) invalid();
  } else {
    object(trigger, ['kind', 'block']);
    if (trigger.kind !== 'blocked') invalid();
    deadlineTriggerBlock(trigger.block);
  }
  if (raw.rule !== null) deadlineRule(raw.rule);
  if (raw.arithmetic !== null) {
    const a = raw.arithmetic;
    object(a, ['rule', 'anchor', 'outcome', 'trace']);
    deadlineRule(a.rule);
    if (!same(a.rule, raw.rule) || !same(factTime(a.anchor), a.anchor)) invalid();
    deadlineOutcome(a.outcome);
    if (!Array.isArray(a.trace) || a.trace.length > 2) invalid();
    a.trace.forEach(deadlineTrace);
  }
  if (raw.due_at !== null) deadlineInstant(raw.due_at);
  if (!Array.isArray(raw.blocks) || raw.blocks.length > 20) invalid();
  raw.blocks.forEach(deadlineEvaluationBlock);
  return raw;
}
