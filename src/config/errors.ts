const MAX_LENGTH = 140

export function friendly(raw: string): string {
  const text = raw.replace(/^Error:\s*/i, '').trim()
  if (/the model did not answer within \d+ seconds/i.test(text))
    return 'The Analysis model took too long. Retry, or check Infer for a busy or loading model.'
  if (/could not reach the model|connection refused|error sending request|timed out/i.test(text))
    return "Can't reach the model server. Is it running?"
  if (/401|403|api key|unauthori[sz]ed|forbidden/i.test(text))
    return 'The model server rejected the API key.'
  const answered = /answered (\d{3})/i.exec(text)
  if (answered) return `The model server answered ${answered[1]}. Try again, or check its log.`
  const brace = text.indexOf('{')
  const plain = (brace > 0 ? text.slice(0, brace) : text).replace(/[:\s]+$/, '').trim() || text
  return plain.length > MAX_LENGTH ? `${plain.slice(0, MAX_LENGTH - 1)}…` : plain
}

export function noticeMillis(text: string): number {
  return Math.min(12_000, Math.max(4_000, text.length * 55))
}