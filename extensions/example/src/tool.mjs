#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

function readRequest() {
  const input = readFileSync(0, 'utf8');
  if (!input.trim()) {
    throw new Error('missing JSON request on stdin');
  }
  return JSON.parse(input);
}

function text(content) {
  return [{ type: 'text', text: content }];
}

function result(id, content, details = {}) {
  return { id, type: 'tool_result', content, details, isError: false };
}

function errorResult(id, message, code = 'example_error') {
  return { id, type: 'error', message, code };
}

function handleEcho(request) {
  const input = request.input ?? {};
  if (input.fail) {
    return errorResult(request.id, 'example_echo was asked to fail', 'requested_failure');
  }
  return result(request.id, text(String(input.text ?? '')), {
    extension: 'example.typescript',
    tool: 'example_echo',
    receivedRunId: request.context?.runId ?? null,
  });
}

function handleWriteDemo(request) {
  const input = request.input ?? {};
  const workspaceRoot = process.env.IMP_EXTENSION_CWD ?? process.cwd();
  const target = resolve(workspaceRoot, String(input.path ?? 'tmp/example-extension/output.txt'));
  const allowedRoot = resolve(workspaceRoot, 'tmp/example-extension');
  if (!target.startsWith(allowedRoot + '/') && target !== allowedRoot) {
    return errorResult(request.id, 'path is outside tmp/example-extension', 'path_denied');
  }
  writeFileSync(target, String(input.text ?? ''), 'utf8');
  return result(request.id, text(`wrote ${target}`), {
    extension: 'example.typescript',
    tool: 'example_write_demo',
    wrote: target,
    directory: dirname(target),
  });
}

function handle(request) {
  if (request.type !== 'tool_call') {
    return errorResult(request.id ?? 'unknown', 'unsupported request type', 'unsupported_request');
  }
  switch (request.tool) {
    case 'example_echo':
      return handleEcho(request);
    case 'example_write_demo':
      return handleWriteDemo(request);
    default:
      return errorResult(request.id, `unknown tool: ${request.tool}`, 'unknown_tool');
  }
}

if (process.argv.includes('--self-test')) {
  const ok = handle({
    id: 'self-test',
    type: 'tool_call',
    tool: 'example_echo',
    input: { text: 'ok' },
    context: { runId: 'self-test' },
  });
  if (ok.content?.[0]?.text !== 'ok') {
    throw new Error('self-test failed');
  }
  console.log('ok');
} else {
  try {
    console.log(JSON.stringify(handle(readRequest())));
  } catch (error) {
    console.log(JSON.stringify(errorResult('call-1', error?.message ?? String(error), 'exception')));
  }
}
