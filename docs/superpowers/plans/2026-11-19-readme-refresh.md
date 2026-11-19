# README Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refresh the root README to document this fork's L06A PCM and native-wake takeover capabilities without tying the Server to specific hardware or an operating system.

**Architecture:** Preserve the upstream project's general introduction and documentation, then add a focused fork-capabilities section and split quick-start guidance between the existing LX06/OH2P path and the firmware-specific L06A path.  The README describes ownership of native KWS, PCM transport, Server VAD/local ASR, and takeover safety without publishing unverified deployment shortcuts.

**Tech Stack:** Markdown, repository-relative links, Git.

**Spec:** `docs/superpowers/specs/2026-11-19-readme-refresh-design.md`

## Global Constraints

- Do not mention N100 or any other Server hardware model.
- Do not state or imply that the upstream repository has stopped maintenance.
- Do not require a particular Server operating system or machine.
- Preserve the original demonstrations, related projects, disclaimer, agreement, and upstream author attribution.
- Keep the existing LX06/OH2P path and present L06A firmware 1.88.55 as a separate verified path.
- State that the user's question is transcribed by Server-side local ASR from PCM tap audio, not Xiaomi ASR.
- Do not add unverified one-command deployment instructions.

## Review Focus

- A reader must not mistake the L06A instructions for LX06/OH2P flashing instructions.
- A reader must not infer that the Server requires a named CPU, host, or operating system.
- Native `小爱同学` KWS and native `我在` must remain owned by the speaker firmware.
- Bridge-side KWS must be described as disabled while Server VAD/local ASR remain enabled.
- Every new relative link must resolve to a tracked repository file.

---

### Task 1: Refresh the root README

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: the existing upstream README structure and `docs/superpowers/specs/2026-11-19-readme-refresh-design.md`.
- Produces: a fork-aware project landing page with separate original and L06A quick-start paths.

- [ ] **Step 1: Record the pre-change validation failures**

Run:

```bash
rg -n '已停止维护|N100' README.md
rg -n 'L06A|pcm_tap|takeover|local ASR' README.md
```

Expected: the stopped-maintenance warning is found, and the fork capability terms are absent.

- [ ] **Step 2: Update `README.md`**

Remove the stopped-maintenance warning.  Add a concise fork-capabilities section near the introduction, including the L06A PCM tap, native KWS/native `我在`, Client takeover, Server VAD/local ASR, and `blocked`/`takeover` safety modes.  Add a compact text flow and split Quick Start into the existing LX06/OH2P path and a separate L06A 1.88.55 path linking to tracked source/design files.  Preserve the existing demonstrations and legal sections.

- [ ] **Step 3: Validate forbidden wording and required ownership boundaries**

Run:

```bash
! rg -n '已停止维护|N100' README.md
rg -n 'L06A|pcm_tap|小爱同学|我在|VAD|本地 ASR|blocked|takeover' README.md
```

Expected: the forbidden search has no matches; every required capability or ownership term has at least one match.

- [ ] **Step 4: Validate new relative links and Markdown diff**

Extract each newly added repository-relative Markdown target from the diff and confirm it exists with `test -e`.  Then run:

```bash
git diff --check
git diff -- README.md
```

Expected: every target exists, `git diff --check` is silent, and the diff contains only the approved README refresh.

- [ ] **Step 5: Commit and push**

```bash
git add README.md
git commit -m "docs: document L06A native wake takeover"
git push fork main
```

Expected: `fork/main` advances to the new documentation commit and the worktree is clean.
