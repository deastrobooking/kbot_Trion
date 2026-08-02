# Autonomy Levels & Skill Promotion — v1

Formalization of the GITAI-style autonomy ladder, extended with field-robot level definitions (human attention required, time between interventions). Every skill in `trion-skills` declares which levels it supports; a skill operates at a level only after meeting that level's promotion criteria with recorded evidence.

## Levels

| Level | Name | Robot does | Human does | Time between interventions | Safety envelope |
| --- | --- | --- | --- | --- | --- |
| **L0** | Manual teleop | Executes joint/Cartesian commands directly | Drives continuously | n/a (hands-on) | Hard limits only (joint, velocity, torque) — always enforced below the policy layer |
| **L1** | Shared control | Handles trajectory, force limits, grasp closure for an operator-specified intent ("grasp this handle") | Specifies intents, monitors continuously | ~minutes | L0 limits + skill-level force envelopes |
| **L2** | Scripted, human-triggered | Executes a predefined skill script end-to-end with safety monitoring; no mid-skill commands needed | Triggers the script, watches, can abort | ~1 hour | L1 + scripted abort conditions |
| **L3** | Gated autonomy | Executes multi-step skills, pausing at defined gates (before contact, before torque, before release); can replan within the skill | Approves each gate (commit-window command class, TR-P4-042c) | ~hours | L2 + gate preconditions verified before request |
| **L4** | Fully autonomous | Executes end-to-end, proposes replans on anomalies | Monitors only; approves replans and reviews before/after | ~days | L3 + anomaly-response behaviors verified |
| **L5** | Adaptive / learning | Improves skill strategies from accumulated experience (e.g., refining insertion approach, reducing force peaks) **within frozen safety constraints** | Reviews strategy changes before deployment | Extended operation | L4 — learned changes never touch the safety envelope; envelope updates are ordinary reviewed releases |

Phase focus: L0–L3 are the Phase 1–2 targets; L4 is the Phase 3 target; L5 is Phase 4+ and only ever inside the reviewed safety envelope.

## Implementation status

The Phase 2 executor now enforces registered autonomy ceilings during whole-plan validation and requires an authenticated Operator or Supervisor at every L3 gate. The initial code-level ceilings are deliberately conservative: `inspect_fixture` and `actuate_handle` are L2, and `mate_connector` is L1. These are execution envelopes, not promotion claims; motion implementations and the evidence campaigns below remain open. See [TASK_PLAN_FORMAT.md](TASK_PLAN_FORMAT.md).

## Promotion criteria

A skill is promoted from level N to N+1 only when **all** of the following are recorded:

1. **Sim campaign:** ≥ 50 successful runs at level N+1 behavior in the digital twin, including the skill's fault-injection scenarios, with success rate ≥ 95% and zero safety-envelope violations.
2. **Hardware campaign:** ≥ 10 successful hardware runs at level N (human still in the loop at the current level) with zero safety interventions.
3. **FDIR coverage:** every FDIR entry the skill can trigger (per its task mapping in [WORKSITES_AND_TASKS.md](WORKSITES_AND_TASKS.md)) has been exercised in sim with the skill active.
4. **Safety case check:** the [PHA.md](PHA.md) hazards and [SAFETY_CASE.md](SAFETY_CASE.md) goals the skill touches have current evidence; no open hazard actions.
5. **Review:** promotion is a reviewed, versioned change (PR) linking the evidence: campaign metrics, event logs, videos.

Demotion is immediate and unilateral: any safety-envelope violation or FDIR escalation attributable to the skill drops it one level pending investigation.

## Per-skill specification template

Each skill in `trion-skills` carries a spec section with exactly these fields:

```markdown
### Skill: <name>
- Supported levels: <current> (target <level> by Phase <n>)
- Description: <one sentence>
- Preconditions: <perception health, localization, no active faults, …>
- Human gates (L3): <numbered gates — e.g., approve approach / approve contact / approve torque>
- Force/torque envelope: <limits and abort thresholds>
- FDIR entries in scope: <F-xx list>
- Promotion evidence: link to EVIDENCE.md
```

### Example — `grasp_handle`
- **Supported levels:** L0–L2 (target L3 by Phase 2)
- **Description:** Approach and grasp a canonical handle fixture (task T-02).
- **Preconditions:** handle recognized and localized in the worksite map; perception health nominal; no active faults in arm joints.
- **Human gates (L3):** 1 — approve approach trajectory; 2 — approve contact initiation.
- **FDIR entries in scope:** F-01, F-04, F-05, F-09, F-10.

### Example — `mate_connector`
- **Supported levels:** L1–L2 (target L3 by Phase 3)
- **Description:** Align and mate an electrical connector to spec (task T-03) — the canonical ISAM contact task.
- **Preconditions:** both connector halves localized; insertion axis known; force/torque sensing available.
- **Human gates (L3):** 1 — approve final alignment; 2 — approve insertion start.
- **Force envelope:** peak insertion force per fixture spec; abort on persistent misalignment (H-02).
- **FDIR entries in scope:** F-01…F-07, F-09, F-10.

## Evidence & logging

Each skill keeps a promotion record in its source tree:

```
trion-skills/<skill>/EVIDENCE.md
  - current level, history of promotions/demotions (dated)
  - links to campaign metric files, event logs (JSONL), force/torque profiles, videos
  - human interventions and reasons
```

Per attempt, log: success/failure, force/torque profiles, FDIR events, and human interventions with reasons — this data justifies promotions, improves policies (L4/L5), and populates post-mission reports.

The `trion-skills` executor enforces levels at runtime: a plan step requesting a skill above its recorded level is rejected at plan-validation time, not at execution time. Mission control will own upload and operator presentation once that crate is implemented.

## Relationship to operations

Early missions run most skills at L1–L2 with heavy oversight; mid-term, flagship skills reach L3 with clear gates; long-term, selected routine-maintenance skills run at L4 with humans approving plans and reviewing results.
