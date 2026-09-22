export interface ToolDefinition {
  name: string
  description: string
  aliases?: string[]
  parameters: {
    type: 'object'
    properties: Record<string, any>
    required?: string[]
  }
  handler: (args: Record<string, any>) => void | Promise<void>
}

class ToolRegistry {
  private tools = new Map<string, ToolDefinition>()

  register(tool: ToolDefinition): () => void {
    this.tools.set(tool.name, tool)
    return () => this.tools.delete(tool.name)
  }

  // Register multiple tools for a single 3D item
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

    // Heuristic fallbacks for small models
    for (const tool of this.tools.values()) {
      if ((clean.includes('print') || clean.includes('paper')) && tool.name === 'print_document') return tool
      if (clean.includes('scan') && tool.name === 'scan_document') return tool
      if ((clean.includes('lamp') || clean.includes('light')) && tool.name === 'set_lighting') return tool
      if ((clean.includes('rotate') || clean.includes('chair')) && tool.name === 'rotate_item') return tool
    }

    return undefined
  }

  async dispatch(rawToolCall: any): Promise<void> {
    const item = Array.isArray(rawToolCall) ? rawToolCall[0] : rawToolCall
    if (!item || !item.name) throw new Error('Invalid tool call structure received')

    const tool = this.resolveTool(item.name)
    if (!tool) {
      console.warn(`[ToolRegistry] Unknown tool: "${item.name}"`)
      return
    }

    const args = item.arguments || item.parameters || {}
    await tool.handler(args)
  }
}

export const globalTools = new ToolRegistry()