# Interface requests and contract change requests

One file per request, named `<yourlane>-<topic>.md` (or `<yourlane>-ccr-<topic>.md` for a contract change). ARCH routes it, answers in the same file, and marks it `Status: ANSWERED` or `DONE`.
The rules are in `docs/swarm/RULES.md` sections 4 and 5; the template:

```
From: <lane>   To: <lane>   Needed by: <milestone or date>   Status: OPEN
What I need: ...
Why (which test or deliverable it unblocks): ...
What I will do meanwhile (stand-in; PROVISIONAL decision): ...
--- ARCH answer (date): ...
```

A contract change request adds: the exact type change you propose, and who else it affects.
