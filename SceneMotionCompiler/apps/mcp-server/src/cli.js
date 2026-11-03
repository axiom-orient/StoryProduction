#!/usr/bin/env node
import { serveStdio } from '@modelcontextprotocol/server/stdio';
import { createSmcMcpServer } from './server.js';
import { parseMcpLaunchConfig } from './launch-config.js';

let handle;

try {
  const config = parseMcpLaunchConfig(process.argv.slice(2));
  handle = serveStdio(
    () => createSmcMcpServer(config),
    {
      legacy: 'reject',
      onerror: (error) => writeDiagnostic('MCP_TRANSPORT_ERROR', error)
    }
  );
} catch (error) {
  writeDiagnostic(typeof error?.code === 'string' ? error.code : 'MCP_STARTUP_FAILED', error);
  process.exitCode = 1;
}

if (handle) {
  let closing = false;
  const close = async (signal) => {
    if (closing) return;
    closing = true;
    try {
      await handle.close();
    } catch (error) {
      writeDiagnostic('MCP_SHUTDOWN_FAILED', error);
      process.exitCode = 1;
    }
    if (signal && process.exitCode === undefined) process.exitCode = 0;
  };

  process.once('SIGINT', () => { void close('SIGINT'); });
  process.once('SIGTERM', () => { void close('SIGTERM'); });
}

function writeDiagnostic(code, error) {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`[smc-mcp] ${code}: ${message}\n`);
}
