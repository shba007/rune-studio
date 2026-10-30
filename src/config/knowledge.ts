export type KnowledgeSection = 'overview' | 'goals' | 'tasks' | 'personalities' | 'help'
export interface KnowledgeConcept {
  id: string
  title: string
  status: 'Available' | 'Partial' | 'Planned' | 'Connected service'
  x: number
  y: number
  meaning: string
  example: string
  today: string
  section?: KnowledgeSection
}
export const knowledgeConcepts: KnowledgeConcept[] = [
  {
    id: 'knowledge',
    title: 'Workspace',
    status: 'Available',
    x: 260,
    y: 165,
    meaning:
      'Your workspace for understanding goals, reviewing tasks and evidence, and managing Rune’s communication styles.',
    example: 'Review a calculation, replay a capture, or edit your fitness coach in one workspace.',
    today:
      'Contains this overview, Goals, Tasks with embedded Preview, and Personalities. This graph explains the product; it is not a map of private records or a claim that every planned connection exists.',
    section: 'overview',
  },
  {
    id: 'request',
    title: 'Your request',
    status: 'Available',
    x: 450,
    y: 65,
    meaning: 'What you type or say. Rune classifies the intended scope as a goal, task, or step.',
    example:
      '“Be physically fit” is a goal; “calculate 14 + 27” is a task; “add this line to that task” is a step.',
    today:
      'Classification is automatic by default. Settings can require confirmation every time; ambiguous input asks for clarification. Fixed-choice classification has three options; goal questions allow a custom answer.',
  },
  {
    id: 'goals',
    title: 'Goals',
    status: 'Partial',
    x: 450,
    y: 350,
    meaning:
      'Longer-term outcomes with a clear starting point, desired result, horizon, resources, and constraints. One ambiguous request may contain several outcomes.',
    example: '“Save 10,000 by year end” or “be fit enough to jog comfortably.”',
    today:
      'Rune asks focused questions with ranked options, then asks you to approve the goal definition. Protected storage requires opt-in. Add tasks explicitly and navigate between a goal and its children. Plan progress counts completed created or due work; outcome progress uses your reported starting, current and target values. Saving a preferred pathway does not create tasks or schedules.',
    section: 'goals',
  },
  {
    id: 'tasks',
    title: 'Tasks',
    status: 'Available',
    x: 650,
    y: 165,
    meaning:
      'Bounded work that produces a result. A request can contain independent tasks, each with sequential steps.',
    example:
      'Calculate an average, translate a sentence, and type a phrase in Notepad as three independent tasks.',
    today:
      'Create manual activities, Rune-executed work or Visual Analysis tasks, optionally linked to an approved goal. The activity timeline shows previews, schedules and history. One-time, daily, weekly and monthly reminders ask before each run. Rune must be open; after time away, overdue occurrences become one pending check-in. Independent execution may run concurrently; device interaction remains exclusive. Old history stays standalone until explicitly linked.',
    section: 'tasks',
  },
  {
    id: 'steps',
    title: 'Steps & attempts',
    status: 'Available',
    x: 810,
    y: 315,
    meaning:
      'A step is one action or reasoning operation within a task. An attempt is one execution of work belonging to that task.',
    example:
      '“Open Notepad → type this text → leave it unsaved” stays in order inside one desktop task.',
    today:
      'An explicitly attached step can create another execution attempt under an existing task. Task history lets you review attempts without treating each attempt as an unrelated task.',
    section: 'tasks',
  },
  {
    id: 'categories',
    title: 'Categories',
    status: 'Available',
    x: 90,
    y: 280,
    meaning:
      'Flexible topic labels for goals. They are not a fixed list, execution permissions, or privacy classifications.',
    example: 'Fitness, finances, learning, or another topic relevant to your goal.',
    today:
      'Rune suggests a category during intake. Approved personality assignments match category labels; unmatched categories use the base personality. Privacy consent uses separate protected data classes.',
  },
  {
    id: 'personalities',
    title: 'Personalities',
    status: 'Partial',
    x: 185,
    y: 445,
    meaning:
      'Editable prompts that shape Rune’s tone, questioning, and coaching style for assigned goal categories.',
    example:
      'A fitness coach can encourage consistency; a finance companion can help clarify constraints without claiming credentials.',
    today:
      'Only the base personality ships. You can draft a style with a model verified as local, edit it, and explicitly approve it before saving. An orb symbol identifies the selected style. Profiles are editable files in Studio’s user-data prompts/personalities folder. Permissions and JSON contracts remain protected.',
    section: 'personalities',
  },
  {
    id: 'context',
    title: 'Context & memory',
    status: 'Partial',
    x: 450,
    y: 530,
    meaning:
      'Relevant information that helps Rune understand your current situation, constraints, and earlier answers.',
    example:
      'Your available time, current jogging ability, or the inputs you supplied for a calculation.',
    today:
      'Goal clarification uses the request and your answers. Task execution uses its supplied context and supported dependencies. Saved history does not imply Rune automatically reads every record. Broad app/service context collection and useful long-term memory remain future work.',
  },
  {
    id: 'consent',
    title: 'Consent & storage',
    status: 'Available',
    x: 295,
    y: 635,
    meaning:
      'Your choices about what Rune may save and use. Different kinds of content have different storage rules.',
    example:
      'Allow protected goal conversations while separately controlling capture and recording settings.',
    today:
      'Goal-related decisions use the encrypted Life store after opt-in. Task archives, captures, recordings, logs, and editable personality files are separate; do not assume all are encrypted or share one retention rule. Completed analyses may save captures for replay. Personal records belong in protected conversations, not style prompts.',
  },
  {
    id: 'safety',
    title: 'Safety & conflicts',
    status: 'Partial',
    x: 650,
    y: 635,
    meaning:
      'Permission checks and explicit decisions protect actions and changes when instructions, resources, or saved versions disagree.',
    example:
      'Two edits to the same personality should trigger a conflict instead of silently replacing each other.',
    today:
      'Execution consent, tool restrictions, cancellation, and stale-edit checks exist. Personality saves reject conflicting versions and duplicate category assignments. A universal conflict inbox spanning every domain is planned.',
  },
  {
    id: 'analysis',
    title: 'Visual Analysis',
    status: 'Available',
    x: 805,
    y: 65,
    meaning:
      'An explicitly requested analysis of a selected screen or window, with findings and visual annotations.',
    example:
      'Use /visual analysis, confirm the capture target, and ask about visible objects or body landmarks.',
    today:
      'The selected Analysis model must support the required inputs. Entities, Regions & Context, Structure & Landmarks, and Depth are visibility groups. Geometry depends on the data the model returns; a checkbox cannot create a missing mask, skeleton, or depth map. Saved successful captures can be reviewed under Tasks.',
    section: 'tasks',
  },
  {
    id: 'recording',
    title: 'Capture & recording',
    status: 'Available',
    x: 815,
    y: 480,
    meaning: 'Screenshots and silent video provide evidence of visible work and allow replay.',
    example: 'Record while typing, then inspect the recording alongside task steps.',
    today:
      'Capture quality, recording resolution, and model input resolution are separate controls. A smaller model input can speed inference without requiring a lower-quality saved recording. Audio recording remains deferred.',
    section: 'tasks',
  },
  {
    id: 'review',
    title: 'Results & Preview',
    status: 'Available',
    x: 650,
    y: 450,
    meaning: 'The answer explains what happened; stored evidence helps you check it.',
    example: 'See “41” in the AI bubble, then open the task to inspect its attempts or recording.',
    today:
      'Final task answers appear in the orb’s bubble. Tasks retain results and evidence. Preview is embedded in Workspace, with task timelines and replay where evidence exists. Older history may lack original prompts or captures that cannot be reconstructed.',
    section: 'tasks',
  },
  {
    id: 'models',
    title: 'Models & Infer',
    status: 'Connected service',
    x: 95,
    y: 100,
    meaning:
      'Models interpret requests, produce structured plans and findings, and guide supported execution.',
    example: 'An Analysis model examines an image; an Agent model plans bounded work.',
    today:
      'Studio connects to a configured endpoint. Protected intake and managed-task execution require models verified as local. After protected execution starts, this Studio process keeps helper inference local too and skips optional alternate-endpoint checks. Structured JSON defines planning/reporting; executable actions still use tools. API and external-agent sessions remain separate.',
  },
  {
    id: 'kit',
    title: 'Kit & tools',
    status: 'Connected service',
    x: 85,
    y: 620,
    meaning:
      'Permissioned tool connections can extend Rune’s reach into services and applications.',
    example:
      'A selected read-only tool can support a background task without taking over the desktop.',
    today:
      'Studio connects to Kit’s independently managed loopback HTTP service. Studio does not launch or own Kit. Available tools depend on your connection and restrictions. Broader service integration is not automatically enabled.',
  },
  {
    id: 'pathways',
    title: 'Pathways & reminders',
    status: 'Partial',
    x: 450,
    y: 190,
    meaning:
      'Alternative routes from a clarified goal to manageable work, with trade-offs, time, check-ins, and progress evidence.',
    example:
      'Agree on a feasible fitness plan, then receive an approved 7 AM jogging reminder and a progress question.',
    today:
      'Compare ranked routes using an approved goal and its personality, then save a preferred route. Draft up to seven first-week tasks using that route and existing linked work. Edit each task, mode and suggested schedule, then approve creation individually. Drafts need protected storage consent and verified-local inference. First occurrences stay within the draft week; approved recurrence can continue afterward. Reminders require your action. Visual Analysis still needs fresh capture selection and your completion report. Automatic execution and proactive coaching remain future work.',
  },
]
export const knowledgeLinks = [
  ['request', 'goals', 'may become'],
  ['request', 'tasks', 'may become'],
  ['request', 'steps', 'may attach as'],
  ['knowledge', 'goals', 'organizes'],
  ['knowledge', 'tasks', 'organizes'],
  ['knowledge', 'personalities', 'manages'],
  ['goals', 'categories', 'has a topic'],
  ['goals', 'personalities', 'uses an approved communication style'],
  ['goals', 'models', 'clarified with a verified-local model'],
  ['goals', 'consent', 'requires approval for protected storage'],
  ['goals', 'tasks', 'explicit parent and child links'],
  ['categories', 'personalities', 'selects an approved style'],
  ['goals', 'context', 'clarified using'],
  ['context', 'consent', 'stored according to'],
  ['tasks', 'steps', 'contains'],
  ['tasks', 'review', 'produces results and evidence'],
  ['analysis', 'tasks', 'saves reviewable analysis'],
  ['recording', 'review', 'provides evidence'],
  ['models', 'request', 'interprets'],
  ['kit', 'tasks', 'offers selected tools'],
  ['safety', 'tasks', 'gates execution'],
  ['safety', 'personalities', 'checks conflicting edits'],
  ['goals', 'pathways', 'compares proposed routes'],
  ['pathways', 'tasks', 'proposes first-week work for approval'],
] as const