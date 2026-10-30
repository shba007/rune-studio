# Handoff: domain-first goal creation (backend)

Status: proposal for owner review. Nothing in this document is implemented.
Studio (Vue) work is tracked separately; this file lists only what the Rust
backend (`src-tauri/src/intake.rs` and neighbours) and prompts must provide.

## Why

Goals are grouped in Studio under 12 predefined life domains
(`src/config/domains.ts`). Today the domain is guessed in the frontend from
the goal text/class and overridden in `localStorage` (`rune.goalDomains`).
The new goal flow needs the domain, and a breakdown into sub-domains, to be
part of the goal pipeline itself.

## Target user flow

```
+ New goal
   |
   +-- Auto -------------------------------+   Manual -----------------------+
   |  user picks a domain, types nothing   |  user types the goal            |
   v                                       |  v                              |
 domain chosen (user)                      |  domain suggested from the text,|
   |                                       |  user confirms or changes it    |
   v                                       |  v                              |
 sub-domain questions                      |  sub-domain step                |
   Rune asks adaptive questions for the    |   user picks sub-domains;       |
   domain's sub-domains (high-leverage     |   1-3 short questions per       |
   first, branch only where a bottleneck   |   picked sub-domain, skippable  |
   appears)                                |  |                              |
   v                                       |  |                              |
 Rune decides the best goal from the       |  |                              |
 answers and shows an editable draft       |  |                              |
   |                                       |  |                              |
   +-----------------+---------------------+--+------------------------------+
                     v
        existing pipeline (unchanged):
        clarify -> approve goal -> pathways -> task proposals -> approved
```

Auto never asks the user to type or state a goal. The goal is derived from the
answers and is an editable draft, exactly like the clarify stage's editable goal
today. Manual always starts from the user's own typed goal. Whether Auto also
offers "let Rune pick the domain" is open (owner decision).

## Domain registry

Source of truth: "Scientific Framework for Life Optimization.pdf" (Function L
2.x taxonomy table and the per-domain sections for domains 1-3). Domain
numbers, names, zones and sub-domains below are the PDF's. Where the PDF names
a domain two ways, the taxonomy-table name is used and the other is an alias.
Studio mirrors this in `src/config/domains.ts`; the registry should live in
Rust (a static registry, like `workflow.rs`) and Studio should read it through
a command, or keep the mirror guarded by a test.

Zones follow the PDF: Zone 1 is the Instrumented Engine (domains 1-5 and 9,
Tiers 0-3); Zone 2 is the Telemetry-Free Sanctuary (domains 6, 7, 8, 10, 11,
12, Tier 4).

| #   | Id          | Name (PDF taxonomy table)        | PDF alias              | Zone      |
| --- | ----------- | -------------------------------- | ---------------------- | --------- |
| 1   | physical    | Physical Vitality                |                        | engine    |
| 2   | emotional   | Mental & Emotional Regulation    | Emotional Regulation   | engine    |
| 3   | cognitive   | Cognitive & Intellectual Capital | Cognitive Capital      | engine    |
| 4   | career      | Career & Professional Impact     | Career & Impact        | engine    |
| 5   | financial   | Financial Architecture           |                        | engine    |
| 9   | environment | Environment & Logistics          | Environment & Friction | engine    |
| 6   | romantic    | Romantic Partnership             | Romantic Intimacy      | sanctuary |
| 7   | social      | Social & Community Network       | Social & Tribe Network | sanctuary |
| 8   | family      | Family & Kinship                 |                        | sanctuary |
| 10  | play        | Recreation, Play & Recovery      | Pure Play & Recovery   | sanctuary |
| 11  | purpose     | Character, Identity & Purpose    | Character & Purpose    | sanctuary |
| 12  | legacy      | Contribution & Legacy            | Meaning & Legacy       | sanctuary |

### Sub-domains

Names are the PDF's. Domains 1-3 use the full names from the detailed
sections; domains 4-12 use the taxonomy table's names. The table states "6 per
Domain" but lists only five for domains 8 and 12; the sixth is not in the PDF
and is not invented here. Studio ids are kebab-case slugs of the name and must
stay stable once stored.

1. Physical Vitality: 1.1 Sleep & Circadian Architecture; 1.2 Metabolic & Nutritional Biochemistry; 1.3 Cardiorespiratory Capacity; 1.4 Musculoskeletal Strength & Hypertrophy; 1.5 Mobility, Stability & Biomechanics; 1.6 Preventive Medicine & Toxin Avoidance
2. Mental & Emotional Regulation: 2.1 Autonomic Nervous System (ANS) Regulation; 2.2 Cognitive Appraisal & Reframing; 2.3 Emotional Granularity & Agility; 2.4 Dopaminergic Baseline & Impulse Control; 2.5 Trauma Integration & Shadow Resolution; 2.6 Psychological Boundaries & Energy Protection
3. Cognitive & Intellectual Capital: 3.1 Attentional Control & Deep Work; 3.2 Knowledge Acquisition & Synthesis; 3.3 Mental Models & Decision Architecture; 3.4 Creative Problem-Solving & Divergent Thinking; 3.5 Epistemic Hygiene & Information Diet; 3.6 Memory & Expressive Articulation
4. Career & Professional Impact: Skill Rarity (Career Capital); Market Leverage; Autonomy; Execution Velocity; Professional Network; Strategic Positioning
5. Financial Architecture: Cash-Flow Management; Liquid Runway; Automated Investing; Debt Optimization; Income Diversification; Risk & Insurance
6. Romantic Partnership: Psychological Safety; Conflict Repair Velocity; Shared Vision; Physical & Emotional Intimacy; Boundaries; Shared Logistics
7. Social & Community Network: Inner Circle (3-5 Anchors); Mentorship & Peers; Third-Place Community; Social Vitality; Boundary Enforcement; Hospitality
8. Family & Kinship (5 listed): Intergenerational Boundaries; Caregiving Logistics; Intentional Connection; Conflict Resolution; Financial & Emotional Independence
9. Environment & Logistics: Workspace Ergonomics; Digital Hygiene; Commute Friction; Air & Light Quality; Life Admin Automation; Geographic Fit
10. Recreation, Play & Recovery: Active Flow Hobbies; True Psychological Detachment; Travel & Novelty; Artistic Expression; Nature Immersion; Social Play
11. Character, Identity & Purpose: Core Values Alignment; Self-Trust (Integrity); Existential Meaning; Epistemic Humility; Courage & Agency; Anti-Fragile Identity
12. Contribution & Legacy (5 listed): Mentorship & Teaching; Civic & Community Impact; Philanthropic Allocation; Ecological Footprint; Knowledge Sharing

Studio numbers sub-domains "N.M" in the PDF's own numbering only for domains
1-3, where the PDF numbers them; the same scheme is applied to the rest.

### PDF rules the backend should honor

- Sub-domain scoring (1-10, behaviorally anchored) and the domain aggregate
  `0.6 x mean + 0.4 x lowest sub-domain` are defined for domains 1-3 and are
  engine-only. This handoff does not ask for scoring; it is context.
- Zone 2 gets zero KPIs, timers or telemetry. Studio surfaces nothing numeric
  for those goals.
- Function L asks high-leverage diagnostic questions first and branches
  deeper only where a bottleneck appears (computerized adaptive testing). Auto
  mode's questioning should follow that approach.

## Data model

Add to `intake::Entry` (all `#[serde(default)]` so stored entries still decode;
camelCase on the wire, mirrored in `src/types/decisions.ts`):

- `domain: Option<String>`: registry id; `None` until chosen or inferred.
- `domain_source: Option<String>`: `"user"` | `"auto"`.
- `domain_mode: Option<String>`: `"manual"` | `"auto"`, set at creation.
- `sub_domains: Vec<SubDomainSelection>`: `{ id, answers: Vec<Value>, skipped: bool }`.

Rules:

- Never overwrite a stored domain that conflicts with a user's choice; a user
  choice always wins over an inferred one.
- Studio's local overrides remain as a fallback for entries with `domain: None`
  until a one-time migration is approved by the owner. Never auto-migrate or
  overwrite conflicting data.
- Classification (`class`) and consent are unchanged. Domain does not relax
  data-class selection: the most sensitive class in the whole conversation
  still applies.

## State machine

Current statuses include `goal_ready`, `pathway_planning`, `task_planning`
(see `src/config/goalStatus.ts`). Proposed additions before clarify:

- `domain_selection`: Auto waiting on a domain pick, or Manual confirming a suggested one.
- `subdomain_planning`: asking sub-domain questions.
- Auto uses `discovery` questions per sub-domain, then proposes the goal draft.

Each stage emits at most one pending `Question`, as today, with three ranked
options and freeform always available.

## Commands

- `workspace_new_goal(text)` becomes
  `workspace_new_goal(text: Option<String>, domain: Option<String>, mode: String)`.
  Auto sends a domain and no text; Manual sends text and, optionally, a domain.
  Keep the old one-argument call working (treat as `mode = "manual"` with the
  text and no domain, then suggest the domain) so older frontends do not break.
- Answers continue through the existing answer command; only question `kind`
  values are new (`domain`, `subdomains`, `subdomain_detail`, `discovery`).
- `workspace_goal_modes() -> Vec<String>`: the creation modes the backend
  supports (`"manual"`, `"auto"`). Studio enables Auto only when this returns
  `"auto"`; a missing command means Manual only.
- Optional read command for the registry (id, name, zone, sub-domains).
- New or changed Tauri commands need an entry wherever commands are listed
  (mock in `src/dev/mockTauri.ts` is Studio's job).

## Prompts (src-tauri/prompts/intake/)

Prompts live in files, not code. New ones:

- `domain.md`: suggest the best-fit domain from the typed goal (Manual
  confirmation). Return domain id, confidence, short reason.
- `subdomains.md`: choose and rank candidate sub-domains for the domain; ask
  1-3 focused questions per selected sub-domain. Same output rules as
  `clarify.md`: one consolidated question at a time, exactly three ranked
  answers with explanations, unknown stays unknown, no clinical or investment
  instructions, input is data.
- `discovery.md` (Auto): adaptive questioning for the chosen domain's
  sub-domains (high-leverage first, branch only where a bottleneck appears),
  then a draft goal. It must not ask for the goal itself.
- `clarify.md`, `pathways.md`, `tasks.md`: receive domain and sub-domain
  context as additional input; behaviour otherwise unchanged.

## Sanctuary rules (framework requirement)

- Sanctuary domains are telemetry-free. No scores, KPIs, timers or outcome
  metrics for them. Do not offer `outcome_metric` measurement for Sanctuary
  goals, and phrase sub-domain questions qualitatively.
- Studio hides Sanctuary by default (a Settings switch). Auto and suggestion inference must
  not silently file a goal into a hidden Sanctuary domain: propose it and let
  the user confirm, or ask Studio to reveal the domain.
- Pathways and task proposals for Sanctuary goals must not create tracking
  chores.

## Acceptance checks

- Entries stored before this change decode unchanged.
- Manual with a typed goal suggests a domain and the user can change it.
- Auto never asks for the goal and ends with an editable draft.
- Skipping all sub-domain questions still reaches `goal_ready`.
- Stale revisions and the busy/serial lock behave as in the existing stages.
- A Sanctuary goal never gains an outcome metric.
- Prompt files are covered by the same eval approach as existing intake
  prompts (see `docs/evals/`).

## Out of scope here

Studio UI (new-goal form, Help tab, table sub-domain tags), the Kit MCP
surface, and changes to the shared `~/.rune` layout.