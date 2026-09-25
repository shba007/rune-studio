export interface ToolDefinition {
  name: string
  description: string
  aliases?: string[]
  parameters: {
    type: 'object'
    properties: Record<string, unknown>
    required?: string[]
  }
  handler: (args: Record<string, unknown>) => void | Promise<void>
}

interface RawToolCallItem {
  name?: string
  arguments?: string | Record<string, unknown>
  parameters?: string | Record<string, unknown>
}

class ToolRegistry {
  private tools = new Map<string, ToolDefinition>()

  register(tool: ToolDefinition): () => void {
    this.tools.set(tool.name, tool)
    return () => this.tools.delete(tool.name)
  }

  registerMany(tools: ToolDefinition[]): () => void {
    const unregs = tools.map((t) => this.register(t))
    return () => unregs.forEach((fn) => fn())
  }

  unregister(name: string) {
    this.tools.delete(name)
  }

  getSchema() {
    return Array.from(this.tools.values()).map(({ name, description, parameters }) => ({
      name,
      description,
      parameters,
    }))
  }

  private resolveTool(rawName: string): ToolDefinition | undefined {
    if (this.tools.has(rawName)) return this.tools.get(rawName)
    const clean = rawName.toLowerCase().replace(/[-_\s]/g, '')

    for (const tool of this.tools.values()) {
      if (tool.name.toLowerCase().replace(/[-_\s]/g, '') === clean) return tool
      if (tool.aliases) {
        for (const alias of tool.aliases) {
          if (alias.toLowerCase().replace(/[-_\s]/g, '') === clean) return tool
        }
      }
    }

    for (const tool of this.tools.values()) {
      if ((clean.includes('print') || clean.includes('paper')) && tool.name === 'print_document')
        return tool
      if (clean.includes('scan') && tool.name === 'scan_document') return tool
      if ((clean.includes('lamp') || clean.includes('light')) && tool.name === 'set_lighting')
        return tool
      if ((clean.includes('rotate') || clean.includes('chair')) && tool.name === 'rotate_item')
        return tool
    }

    return undefined
  }

  async dispatch(rawToolCall: unknown): Promise<void> {
    const item = (Array.isArray(rawToolCall) ? rawToolCall[0] : rawToolCall) as
      | RawToolCallItem
      | null
      | undefined
    if (!item || typeof item.name !== 'string')
      throw new Error('Invalid tool call structure received')

    const tool = this.resolveTool(item.name)
    if (!tool) {
      console.warn(`[ToolRegistry] Unknown tool: "${item.name}"`)
      return
    }

    let args: Record<string, unknown> = {}
    const rawArgs = item.arguments || item.parameters || {}
    if (typeof rawArgs === 'string') {
      try {
        args = JSON.parse(rawArgs) as Record<string, unknown>
      } catch {
        console.error('[ToolRegistry] Failed to parse tool call arguments:', rawArgs)
      }
    } else if (typeof rawArgs === 'object' && rawArgs !== null) {
      args = rawArgs as Record<string, unknown>
    }

    await tool.handler(args)
  }
}

export const globalTools = new ToolRegistry()