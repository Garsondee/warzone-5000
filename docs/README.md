# Documentation

Start with the **brief**, then your **lane brief**. Everything here is written for a capable reader who has not seen the project, including the owner (a computer-graphics person who wants to understand the principles).

| Read | What it is |
|---|---|
| [brief/BRIEF.md](brief/BRIEF.md) | what we are building and why, in two pages; [NON-GOALS](brief/NON-GOALS.md), [GLOSSARY](brief/GLOSSARY.md), [NOT-MODELLED](brief/NOT-MODELLED.md) |
| [architecture/](architecture/) | [LAYERS](architecture/LAYERS.md) (crates and owners), [CONTRACTS](architecture/CONTRACTS.md) (the interfaces), [UNITS-AND-FRAMES](architecture/UNITS-AND-FRAMES.md) (the signs), [DETERMINISM](architecture/DETERMINISM.md) |
| [decisions/](decisions/) | the ADRs (why we chose what we chose) and the [decision queue](decisions/QUEUE.md) (what the owner still has to decide, each with a default) |
| [swarm/](swarm/) | how the parallel lanes work: [RULES](swarm/RULES.md), [MERGE-GATE](swarm/MERGE-GATE.md), [OWNERSHIP](swarm/OWNERSHIP.md), [GUARDRAILS](swarm/GUARDRAILS.md), the lane briefs in [swarm/lanes/](swarm/lanes/), live state in [STATE](swarm/STATE.md) |
| [validation/](validation/) | how we check the vehicles against real ones: [METHOD](validation/METHOD.md), the [Design Impact Matrix](validation/IMPACT-MATRIX.md), the dossier template |
| [theory/](theory/) | plain-language notes on the principles behind each module (one per lane, written as the lane builds) |
| [dev-environment.md](dev-environment.md) | what a cloud session can and cannot run |
| [research/](research/) | the Warzone 2100 research notes (notes only; no code or assets are copied, GPL) |
| [archive/prototype-v0/](archive/prototype-v0/PROTOTYPE-INDEX.md) | the superseded prototype: documents, decision log, and the port ledger |
