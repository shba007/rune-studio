export const historyFixture = [
  { revision: 1, atMs: 1000, snapshot: { request: { historyId: 'request' }, tasks: [] } },
  {
    revision: 3,
    atMs: 31000,
    snapshot: {
      request: { historyId: 'request', finishedAt: 31000 },
      tasks: [
        {
          id: 1,
          text: 'Calculate',
          kind: 'background',
          phase: 'done',
          startedAt: 2000,
          finishedAt: 10000,
          steps: [{ text: 'Sum', phase: 'done', startedAt: 2000, finishedAt: 10000 }],
        },
        {
          id: 2,
          text: 'Translate',
          kind: 'background',
          phase: 'failed',
          startedAt: 2000,
          finishedAt: 13000,
          steps: [{ text: 'Translate', phase: 'failed', startedAt: 2000, finishedAt: 13000 }],
        },
        {
          id: 3,
          text: 'Notepad',
          kind: 'desktop',
          runId: 'desktop',
          phase: 'cancelled',
          startedAt: 6000,
          finishedAt: 26000,
          steps: [{ text: 'Type', phase: 'cancelled', startedAt: 11000, finishedAt: 26000 }],
        },
      ],
    },
  },
]