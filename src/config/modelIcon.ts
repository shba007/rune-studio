export function modelIcon(model: string) {
  const families: [RegExp, string][] = [
    [/qwen|qwq/i, 'qwen'],
    [/deepseek/i, 'deepseek'],
    [/gemma/i, 'gemma'],
    [/gemini/i, 'gemini'],
    [/claude/i, 'claude'],
    [/llama|(?:^|\/)meta(?:[-/]|$)/i, 'meta'],
    [/mistral|mixtral|ministral|codestral|devstral/i, 'mistral'],
    [/(?:^|[/-])phi[-\d]|microsoft/i, 'microsoft'],
    [/gpt|chatgpt|openai|(?:^|\/)o[134](?:[-/]|$)/i, 'openai'],
  ]
  return `/model/${families.find(([pattern]) => pattern.test(model))?.[1] ?? 'model'}.svg`
}