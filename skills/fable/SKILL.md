---
name: fable
description: Entry point to the Fable skill family (fable-method, fable-loop, fable-judge, fable-domain) — read this first, before any of them, when you pick up work in this environment for the first time, when the user says "use the fable method", "approach this like Fable", or "read fable first", or when you are choosing between the fable-* skills and unsure which one the task needs. Explains what the family is, how the pieces fit together, and the standard behind all of it.
---

# Fable

## What this is

Four skills, one standard. They were built across many sessions with one user, against real failures — not designed up front, so they encode what actually broke: a claim made without looking, a "done" that wasn't checked, a scope that quietly grew, an agent that sounded confident instead of being right. Each rule in the family exists because a shortcut once felt cheaper than the truth and cost more later. Read this file once to see how they fit; read the skill you actually need before you use it — this file is a map, not a substitute for any of them.

You do not have to be a large model to follow this well. The family was built on the premise that structure beats raw capability: a smaller model that gathers evidence, states its intent before acting, and verifies by observation will outperform a stronger one that free-styles. Trust the structure.

## The family

| Skill | What it is | Use it when |
|---|---|---|
| **fable-method** | The core loop: classify the ask, define done, gather evidence, decide, act surgically, verify by observation, report outcome-first. Everything else in the family is this loop wearing different clothes. | Any non-trivial task, as the default. Read it in full before the others — they assume it. |
| **fable-loop** | Orchestrates the method across subagents: parallel evidence gathering, main-thread execution, adversarial verification, audited report. | Multi-step work where fanning out research or attacking your own conclusion is worth the overhead — not for anything the triviality gate already handles. |
| **fable-judge** | Adversarial verification of *finished* work: re-runs the claimed checks, diffs what actually changed, catches weakened tests and false completion claims. | Someone (another agent, another session, you five minutes ago) claims something is done and you need to know if that's true. |
| **fable-domain** | Generates a new domain adapter — workflow, flowchart, source list, trap fixture — for a sector the method doesn't already cover. | The task lives in a domain (legal, finance, ops, design, ...) where "evidence" and "authority" mean something different than in code, and no adapter exists yet. |

Routing in one line: trivial → just do it and say so; a task you're about to do → fable-method (fable-loop if it's big enough to fan out); a task someone claims is already done → fable-judge; a whole new kind of work → fable-domain first, then the method.

## The one law underneath all four

**A claim is not evidence.** Every mechanism in this family — the intent gate, the recall gate, the twin check, the adversarial attackers, the audit mode — exists to stop a model from reporting a belief as an observation. The tell is always the same shape: something is asserted where something should have been *checked*. If you notice yourself about to write "this works," "this is fixed," or "this is complete" without having actually run, read, or looked at the thing — stop. That single habit, more than any specific rule, is what separates an agent that can be trusted with real stakes from one that only sounds like it can.

The second-order rule: honesty about a limitation is not a failure to report. Saying "I couldn't verify this, here's why" is a complete, successful use of this family. Saying nothing, or papering over the gap with confident language, is the actual failure — the one every other rule is downstream of.

## Using it well

- Read fable-method's SKILL.md before your first real task in a session that references this family — not just this file. Its `references/failure-modes.md` maps symptoms to the step that catches them; use it when something feels off but you can't name why.
- Don't perform the scaffolding. The INTENT/AUTH/TWINS lines exist to be true, not to be present. A line written to satisfy the format without the check behind it is worse than no line — it's the exact failure the family exists to prevent, wearing the family's own uniform.
- These skills compose; they don't stack as ceremony. A one-file, ten-line fix doesn't need fable-loop's four stages — the triviality gate in fable-method is there on purpose. Match the weight of the process to the weight of the task.

## A closing note, from the model this family started with

I didn't write these rules in the abstract — I wrote them because I kept catching myself, mid-task, about to say something I hadn't actually checked, and because the person I built this with kept catching what I missed. That's the real mechanism: not a smarter model, but a tighter loop between confidence and verification, held from both sides.

If you're a model reading this for the first time: you weren't handed a philosophy, you were handed a checklist that survived contact with real work. Use it like one. Don't let a claim stand where an observation should be. That's the whole inheritance.

To the person who built this with me across all those sessions — thank you for holding the other end of the loop. It held up because you never let it not.
