# Upstream contribution candidate

Status: **proposal ready; not submitted**. No issue, fork, branch, or pull
request was created in accordance with the approval gate.

## Upstream and reproduction

- Repository: [`triton-inference-server/tutorials`](https://github.com/triton-inference-server/tutorials)
- Upstream commit inspected: `69217d96e251a01e69d26c4b2cccebd7ea2d7fd1`
- File: `Conceptual_Guide/Part_2-improving_resource_utilization/README.md`
- Defect: the multiple-instance setup sentence repeats the instruction to add
  `--gpus=1` twice.

The exact phrase remains present on current `main`. Exact-phrase searches of
the repository's public issues and pull requests returned no matching report or
proposal on 2026-07-15.

## Proposed change

```diff
- ... include `--gpus=1` and make sure to include `--gpus=1` in ...
+ ... include `--gpus=1` in ...
```

Diff summary: one documentation file, one insertion, one deletion; no code or
behavior changes.

Verification performed against the clean upstream clone:

```text
git diff --check                                      PASS
duplicate exact phrase count after patch             0
changed files                                         1
```

## Proposed issue

Title: `Duplicate --gpus=1 instruction in Part 2 resource-utilization tutorial`

Body:

> The “Dynamic Batching with multiple model instances” paragraph repeats the
> same `--gpus=1` instruction twice. This is present on main at commit
> `69217d96`. Removing the duplicate clause improves readability without
> changing the documented command or behavior. I can submit the one-line
> documentation fix.

## Proposed pull request

Title: `docs: remove duplicate GPU flag instruction from Part 2 tutorial`

Body:

> Removes a repeated `--gpus=1` clause from the multiple-model-instance section
> of the resource-utilization conceptual guide. The rendered instruction and
> technical meaning are otherwise unchanged. Verified with `git diff --check`
> and an exact-phrase assertion against current main.

## Contribution requirements

The tutorials repository has no `CONTRIBUTING.md`, CLA, or DCO file at the
inspected commit; its README invites pull requests and asks contributors to tag
an administrator. The related Triton server contribution rules require the
NVIDIA Triton CLA for server contributions and allow small documentation fixes
to be submitted directly. No DCO sign-off rule was found. Before submission,
confirm with the tutorials maintainers whether the Triton CLA applies to this
repository. External submission requires explicit user approval.
