# Project Guidelines (Karpathy Guidelines)

Behavioral guidelines to reduce common LLM coding mistakes during the development of this project, derived from [Andrej Karpathy's observations](https://x.com/karpathy/status/2015883857489522876) on LLM coding pitfalls.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

---

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them — don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

---

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: *"Would a senior engineer say this is overcomplicated?"* If yes, simplify.

---

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it — don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

**The test:** Every changed line should trace directly to the user's request.

---

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```text
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

## 5. Mandatory Research-First Engineering Protocol (FriedChip+)

**Core principle: FriedChip+ should build on established firmware engineering, not repeatedly reinvent it. Research first. Compare. Validate. Reuse responsibly. Then implement.**

This requirement applies to every future implementation, correction, refactor, parser, decoder, validator, extractor, security assessment, and test suite.

### 1. Research Before Implementation
Before creating new functionality or modifying existing implementation logic:
- Inspect the project's `research/` directory.
- Identify relevant cloned upstream repositories under `research/third_party/`.
- Inspect actual source code, tests, documentation, commit history where useful, and license files.
- Read the existing research reports and adoption/comparison matrices.
- Identify reusable implementations, algorithms, data structures, validation strategies, and test cases.
- Perform targeted web research to validate findings against authoritative specifications and upstream documentation.
- Do not rely exclusively on README descriptions, repository stars, or previous AI-generated research summaries.

### 2. Reuse Before Reinvention
For every implementation task, explicitly investigate whether an existing upstream project already solves the problem.
Prioritize relevant established projects, including:
- UEFITool / UEFIExtract
- BIOSUtilities
- Fiano
- coreboot / ifdtool
- ME Analyzer
- PSPTool
- CHIPSEC
- Binwalk
- uefi-firmware-parser
- IFRExtractor-RS
- Other relevant repositories already present in `research/third_party/`

Do not automatically introduce new parsers, binary layouts, algorithms, or heuristics when a proven upstream implementation exists.
Reuse compatible components when technically and legally appropriate. When direct reuse is unsuitable, independently implement the concept using the upstream project's documented behaviour and authoritative specifications.

### 3. License and Provenance Requirements
Before adopting code:
- Identify its actual repository license.
- Review applicable file-level licensing and notices.
- Check dependencies and their licenses.
- Record the upstream repository, commit SHA, relevant source paths, and the reason for adoption.
- Preserve required copyright and attribution notices.
- Do not copy code with unclear or incompatible licensing.
- Conceptual inspiration and independent implementations must be distinguished from direct code reuse.

### 4. Mandatory Research Deliverable
Before implementation, produce a concise research assessment containing:
- **Problem:** Exact engineering problem being solved.
- **Upstream references:** Repositories, source paths, commit SHAs.
- **Web references:** Specifications and authoritative documentation.
- **Existing solutions:** Relevant algorithms, structures, and tests.
- **Reuse decision:** Adopt, adapt, independently implement, or reject.
- **Limitations:** Known upstream gaps and risks.
- **License review:** Reuse compatibility and attribution requirements.
- **Proposed design:** Evidence-backed implementation approach.

If research reveals that the existing design is wrong, revise the design before writing code.

### 5. Evidence and Honesty
- Never claim source inspection without actually inspecting the source.
- Never claim an upstream component is reusable without checking its implementation and license.
- Never invent specifications, register layouts, bit positions, or compatibility assumptions.
- Distinguish documented behaviour from assumptions and observed behaviour.
- Record unresolved uncertainties explicitly.
- Prefer `INDETERMINATE` over unsupported positive compatibility claims.
- Use differential tests against trusted upstream implementations when possible.

### 6. Research Availability Failure
If `research/third_party/` contains incomplete clones, placeholders, failed downloads, missing source files, or unavailable repositories:
1. Diagnose the problem.
2. Check repository remotes, commit SHAs, checkout state, and tracked source-file counts.
3. Recover or fetch the missing research material where possible.
4. Do not silently substitute README-only research.
5. Report anything that could not be verified.

Do not claim the research requirement has been fulfilled until the relevant source material has actually been inspected.

### 7. Implementation Gate
No implementation begins until the research assessment has identified the preferred approach and its supporting evidence.
The final implementation report must include:
- Which upstream components or concepts were reused.
- Which approaches were independently implemented.
- Which upstream approaches were rejected and why.
- Relevant licenses and provenance.
- Differential/regression test evidence.
- Remaining gaps and limitations.
