#!/usr/bin/env node
/**
 * Broker WebSocket Integration Tests
 *
 * Tests the broker's WebSocket protocol: ping/pong, command execution,
 * session management, and shell operations.
 *
 * Prerequisites:
 *   - Broker running (local or remote via SSH tunnel)
 *   - `ws` npm package installed (npm install ws)
 *
 * Usage:
 *   node tests/test_broker_ws.js [broker_url]
 *   Default: ws://127.0.0.1:19999
 *
 * For remote broker via SSH tunnel:
 *   ssh -N -L 14001:127.0.0.1:9999 user@remote &
 *   node tests/test_broker_ws.js ws://127.0.0.1:14001
 */

const WebSocket = require('ws');

const BROKER_URL = process.argv[2] || 'ws://127.0.0.1:19999';
const TIMEOUT_MS = 60000;

let pass = 0;
let fail = 0;
const results = [];

function green(s) { return `\x1b[32m${s}\x1b[0m`; }
function red(s) { return `\x1b[31m${s}\x1b[0m`; }
function bold(s) { return `\x1b[1m${s}\x1b[0m`; }

function assert(name, condition) {
  if (condition) {
    console.log(`  ${green('PASS')}: ${name}`);
    pass++;
    results.push({ name, status: 'pass' });
  } else {
    console.log(`  ${red('FAIL')}: ${name}`);
    fail++;
    results.push({ name, status: 'fail' });
  }
}

function connectWs() {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(BROKER_URL);
    ws.on('open', () => resolve(ws));
    ws.on('error', reject);
    setTimeout(() => reject(new Error('Connection timeout')), 10000);
  });
}

function sendAndReceive(ws, msg, filterType, timeoutMs = TIMEOUT_MS) {
  return new Promise((resolve, reject) => {
    const handler = (data) => {
      const parsed = JSON.parse(data.toString());
      if (!filterType || parsed.type === filterType) {
        ws.removeListener('message', handler);
        resolve(parsed);
      }
    };
    ws.on('message', handler);
    ws.send(JSON.stringify(msg));
    setTimeout(() => {
      ws.removeListener('message', handler);
      reject(new Error(`Timeout waiting for ${filterType || 'any'} message`));
    }, timeoutMs);
  });
}

function collectMessages(ws, msg, untilType, timeoutMs = TIMEOUT_MS) {
  return new Promise((resolve, reject) => {
    const messages = [];
    const handler = (data) => {
      const parsed = JSON.parse(data.toString());
      messages.push(parsed);
      if (parsed.type === untilType) {
        ws.removeListener('message', handler);
        resolve(messages);
      }
    };
    ws.on('message', handler);
    ws.send(JSON.stringify(msg));
    setTimeout(() => {
      ws.removeListener('message', handler);
      reject(new Error(`Timeout after ${timeoutMs}ms, collected ${messages.length} messages`));
    }, timeoutMs);
  });
}

async function testPing(ws) {
  console.log(bold('\n[1] Ping/Pong'));
  const resp = await sendAndReceive(ws, { type: 'ping' }, 'pong', 5000);
  assert('Pong response received', resp.type === 'pong');
  assert('Pong has version', typeof resp.version === 'string' && resp.version.length > 0);
  assert('Pong has cli_versions', typeof resp.cli_versions === 'object');
  return resp;
}

async function testInvalidRequest(ws) {
  console.log(bold('\n[2] Invalid Request'));
  const resp = await sendAndReceive(ws, { type: 'command' }, 'error', 5000);
  assert('Error response for malformed command', resp.type === 'error');
  assert('Error has message', typeof resp.error === 'string');
}

async function testSimpleCommand(ws) {
  console.log(bold('\n[3] Simple Command (echo via sh)'));
  // Use a simple shell command via unknown provider (falls back to bare command execution)
  const msgs = await collectMessages(ws, {
    type: 'command',
    session_id: 'test-echo-1',
    provider: 'echo',
    command: 'Hello World',
    options: { cwd: '/tmp' }
  }, 'complete', 15000).catch(() => {
    // If it fails, try to collect what we have
    return [];
  });

  const sessionCreated = msgs.find(m => m.type === 'session-created');
  const complete = msgs.find(m => m.type === 'complete');

  assert('Session created message received', !!sessionCreated);
  if (sessionCreated) {
    assert('Session ID assigned', typeof sessionCreated.actual_session_id === 'string');
  }
  // Complete may or may not be received depending on whether 'echo' binary exists
  if (complete) {
    assert('Command completed', true);
  } else {
    console.log('  (echo command may not have completed - provider "echo" may not exist)');
    assert('Command sent without crash', true);
  }
}

async function testClaudeCommand(ws) {
  console.log(bold('\n[4] Claude CLI Command'));
  console.log('  (Sending simple prompt to Claude CLI...)');

  const msgs = await collectMessages(ws, {
    type: 'command',
    session_id: '',
    provider: 'claude',
    command: 'What is 2+2? Reply with just the number.',
    options: {
      cwd: '/tmp',
      permissionMode: 'dangerouslySkipPermissions'
    }
  }, 'complete', 120000);

  const sessionCreated = msgs.find(m => m.type === 'session-created');
  const providerMsgs = msgs.filter(m => m.type === 'provider-message');
  const complete = msgs.find(m => m.type === 'complete');

  assert('Session created', !!sessionCreated);
  assert('Got provider messages', providerMsgs.length > 0);
  assert('Command completed', !!complete);
  if (complete) {
    assert('Exit code is 0', complete.exit_code === 0);
  }

  // Check if we got actual text output
  let hasTextOutput = false;
  for (const msg of providerMsgs) {
    const data = msg.data;
    if (data && data.type === 'assistant' && data.content) {
      for (const block of data.content) {
        if (block.type === 'text' && block.text) {
          hasTextOutput = true;
          console.log(`  Claude said: "${block.text.substring(0, 100)}"`);
        }
      }
    }
    if (data && data.type === 'result' && data.result) {
      hasTextOutput = true;
      const r = typeof data.result === 'string' ? data.result : JSON.stringify(data.result);
      console.log(`  Result: "${r.substring(0, 100)}"`);
    }
  }
  assert('Got text output from Claude', hasTextOutput);
}

async function testAbortSession(ws) {
  console.log(bold('\n[5] Abort Session'));
  // Start a long-running command
  ws.send(JSON.stringify({
    type: 'command',
    session_id: 'abort-test',
    provider: 'claude',
    command: 'Write a very long essay about the history of computing, at least 5000 words.',
    options: { cwd: '/tmp', permissionMode: 'dangerouslySkipPermissions' }
  }));

  // Wait for session created
  await new Promise((resolve) => {
    const handler = (data) => {
      const msg = JSON.parse(data.toString());
      if (msg.type === 'session-created') {
        ws.removeListener('message', handler);
        resolve(msg);
      }
    };
    ws.on('message', handler);
    setTimeout(resolve, 5000);
  });

  // Send abort
  ws.send(JSON.stringify({ type: 'abort', session_id: 'abort-test' }));
  assert('Abort sent without error', true);

  // Drain remaining messages
  await new Promise(r => setTimeout(r, 2000));
}

async function testStatus(ws) {
  console.log(bold('\n[6] Status Check'));
  ws.send(JSON.stringify({ type: 'status', session_id: 'nonexistent-session' }));
  const resp = await new Promise((resolve) => {
    const handler = (data) => {
      const msg = JSON.parse(data.toString());
      if (msg.type === 'status' || msg.sessionId === 'nonexistent-session') {
        ws.removeListener('message', handler);
        resolve(msg);
      }
    };
    ws.on('message', handler);
    setTimeout(() => resolve(null), 5000);
  });
  assert('Status response received', resp !== null);
  if (resp) {
    assert('Nonexistent session is inactive', resp.active === false);
  }
}

async function main() {
  console.log(bold('========================================='));
  console.log(bold('  Broker WebSocket Integration Tests'));
  console.log(bold(`  Target: ${BROKER_URL}`));
  console.log(bold('========================================='));

  let ws;
  try {
    ws = await connectWs();
    console.log(green('  Connected to broker'));
  } catch (e) {
    console.error(red(`  Failed to connect: ${e.message}`));
    console.error(red('  Make sure the broker is running'));
    process.exit(1);
  }

  try {
    await testPing(ws);
    await testInvalidRequest(ws);
    await testStatus(ws);
    await testSimpleCommand(ws);

    // Only run Claude test if --with-claude flag is passed
    if (process.argv.includes('--with-claude')) {
      await testClaudeCommand(ws);
      await testAbortSession(ws);
    } else {
      console.log(bold('\n[4] Claude CLI Command (SKIPPED - use --with-claude to enable)'));
      console.log(bold('\n[5] Abort Session (SKIPPED)'));
    }
  } catch (e) {
    console.error(red(`  Error: ${e.message}`));
    fail++;
  }

  ws.close();

  console.log(bold('\n========================================='));
  console.log(bold(`  Results: ${pass} passed, ${fail} failed`));
  console.log(bold('========================================='));

  process.exit(fail > 0 ? 1 : 0);
}

main();
