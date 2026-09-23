// A thin MCP client for the Ajisai server in tools/mcp-server. The experiment
// talks to the language only through the same `compute` tool its subject
// agents use, so the grader and the subjects see one engine.

import { Client } from '../../mcp-server/node_modules/@modelcontextprotocol/sdk/dist/esm/client/index.js';
import { StdioClientTransport } from '../../mcp-server/node_modules/@modelcontextprotocol/sdk/dist/esm/client/stdio.js';
import { fileURLToPath } from 'node:url';

const SERVER = fileURLToPath(new URL('../../mcp-server/index.js', import.meta.url));

export async function connect() {
  const transport = new StdioClientTransport({ command: 'node', args: [SERVER] });
  const client = new Client({ name: 'lexicon-emergence', version: '0.1.0' });
  await client.connect(transport);

  async function call(name, args) {
    const result = await client.callTool({ name, arguments: args });
    return JSON.parse(result.content[0].text);
  }

  return {
    compute: (source) => call('compute', { source }),
    check: (source) => call('check', { source }),
    close: () => client.close(),
  };
}

/** The final stack as display strings, bottom to top; null when the run did not end ok. */
export function stackOf(result) {
  return result.status === 'ok' ? (result.stackDisplay ?? []) : null;
}
