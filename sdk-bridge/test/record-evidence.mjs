// Generate a synthetic, shareable protocol transcript with the actual pinned SDK.
import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { OfficialSdkBridge, PROTOCOL_VERSION, sha256 } from '../src/bridge.mjs';
import { offlineFixture } from './fixture-runtime.mjs';
const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'director-sdk-evidence-'));
const transcript = [];
const bridge = new OfficialSdkBridge({ stateDirectory: directory, offlineFixture: offlineFixture('happy'), emit: event => transcript.push({ direction: 'bridge_to_parent', message: event }) });
const request = (requestId, method, attemptId = null, params) => ({ protocolVersion: PROTOCOL_VERSION, requestId, attemptId, method, ...(params ? { params } : {}) });
async function send(message) { transcript.push({ direction: 'parent_to_bridge', message }); const response = await bridge.handle(message); transcript.push({ direction: 'bridge_to_parent', message: response }); }
try {
  await bridge.initialize();
  await send(request('evidence_hello', 'hello'));
  const compiledPrompt = 'Synthetic offline request only. 保留合成原句。';
  const schema = { type: 'object', properties: { ok: { type: 'boolean' }, text: { type: 'string' } }, required: ['ok', 'text'], additionalProperties: false };
  const compiledSchema = JSON.stringify(schema);
  const attempt = { version: 'text-attempt/v2', attemptId: 'evidence_attempt', workflowId: 'synthetic_workflow', logicalNodeId: 'synthetic_node', backend: 'codex_sdk', request: { stage: 'fixture', skill: 'fixture', model: 'gpt-6.1-sol', reasoning: 'medium', input: { synthetic: true }, schema, ruleSnapshot: {} }, semanticRequestHash: 'synthetic-caller-owned-hash', executionFingerprint: 'synthetic-caller-owned-fingerprint', runtimeIdentity: { synthetic: true }, state: 'prepared', threadId: null, turnId: null, lastSequence: 0, rawResult: null };
  await send(request('evidence_execute', 'execute', attempt.attemptId, { attempt, compiledPrompt, compiledSchema, compiledPromptSha256: sha256(compiledPrompt), compiledSchemaSha256: sha256(compiledSchema) }));
  await bridge.active?.task;
  await send(request('evidence_observe', 'observe', attempt.attemptId));
  await send(request('evidence_shutdown', 'shutdown'));
  await fs.writeFile(new URL('../verification/synthetic-protocol.ndjson', import.meta.url), transcript.map(entry => JSON.stringify(entry)).join('\n') + '\n');
} finally { await bridge.disconnect(); await fs.rm(directory, { recursive: true, force: true }); }
