# Autonomy Levels & Skill Promotion — v0

Formalization of the GITAI-style autonomy ladder. Every skill in `trion-skills` declares which levels it supports; a skill operates at a level only after meeting that level's promotion criteria with recorded evidence.

## Levels

| Level | Name | Robot does | Human does | Safety envelope |
| --- | --- | --- | --- | --- |
| **L0** | Manual teleop | Executes joint/Cartesian commands directly | Drives continuously | Hard limits only (joint, velocity, torque) — always enforced below the policy layer |
| **L1** | Shared control | Handles trajectory, force limits, grasp closure for an operator-specified intent ("grasp this handle") | Specifies intents, monitors continuously | L0 limits + skill-level force envelopes |
| **L2** | Scripted, human-triggered | Executes a predefined skill script end-to-end with safety monitoring | Triggers the script, watches, can abort | L1 + scripted abort conditions |
| **L3** | Gated autonomy | Executes multi-step skills, pausing at defined gates (before contact, before torque, before release) | Approves each gate (commit-window command class, TR-P4-042c) | L2 + gate preconditions verified before request |
| **L4** | Fully autonomous | Executes end-to-end, proposes replans on anomalies | Monitors only; approves replans | L3 + anomaly-response behaviors verified |

## Promotion criteria

A skill is promoted from level N to N+1 only when **all** of the following are recorded:

1. **Sim campaign:** ≥ 50 successful runs at level N+1 behavior in the digital twin, including the skill's fault-injection scenarios, with success rate ≥ 95% and zero safety-envelope violations.
2. **Hardware campaign:** ≥ 10 successful hardware runs at level N (i.e., with the human still in the loop at the current level) with zero safety interventions.
3. **FDIR coverage:** every FDIR entry the skill can trigger (per its task mapping in [WORKSITES_AND_TASKS.md](WORKSITES_AND_TASKS.md)) has been exercised in sim with the skill active.
4. **Safety case check:** the [PHA.md](PHA.md) goals touched by the skill have current evidence; no open hazard actions.
5. **Review:** promotion is a reviewed, versioned change (PR) linking the evidence: campaign metrics, event logs, videos.

Demotion is immediate and unilateral: any safety-envelope violation or FDIR escalation attributable to the skill drops it one level pending investigation.

## Evidence record

Each skill keeps a promotion record in its source tree:

```
trion-skills/<skill>/EVIDENCE.md
  - current level, history of promotions/demotions (dated)
  - links to campaign metric files, event logs (JSONL), videos
```

The mission-control plan executor enforces levels at runtime: a plan step requesting a skill above its recorded level is rejected at plan-validation time, not at execution time.
