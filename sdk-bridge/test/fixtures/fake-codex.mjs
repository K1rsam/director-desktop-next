// Synthetic subprocess only. No network libraries, credentials, or actual runtime calls.
import fs from 'node:fs';
const args = process.argv.slice(2);
const arg = flag => args[args.indexOf(flag) + 1];
let prompt = '';
for await (const chunk of process.stdin) prompt += chunk;
const scenario = process.env.DIRECTOR_FIXTURE_SCENARIO;
fs.writeFileSync(process.env.DIRECTOR_FIXTURE_CAPTURE, JSON.stringify({ args, prompt, schema: JSON.parse(fs.readFileSync(arg('--output-schema'), 'utf8')), env: process.env, pid: process.pid }), { mode: 0o600 });
const event = value => process.stdout.write(JSON.stringify(value) + '\n');
const thread = { type: 'thread.started', thread_id: 'synthetic-thread-1' };
const start = { type: 'turn.started' };
const text = { type: 'item.completed', item: { id: 'synthetic-item-1', type: 'agent_message', text: scenario === 'invalid-text-json' ? '{not repaired' : '{"ok":true,"text":"合成测试"}' } };
const complete = { type: 'turn.completed', usage: { input_tokens: 3, cached_input_tokens: 0, output_tokens: 5, reasoning_output_tokens: 1 } };
if (scenario === 'out-of-order') { event(start); event(thread); event(text); event(complete); }
else if (scenario === 'malformed-json') { event(thread); event(start); process.stdout.write('{this is not runtime JSON}\n'); }
else if (scenario === 'early-complete') { event(thread); event(start); event(complete); event(text); }
else if (scenario === 'changed-thread') { event(thread); event({ ...thread, thread_id: 'unexpected-thread' }); }
else if (scenario === 'policy-violation') { event(thread); event(start); event({ type: 'item.started', item: { id: 'tool-1', type: 'command_execution', command: 'synthetic-unexecuted-command', status: 'in_progress' } }); }
else if (scenario === 'failed-after-text') { event(thread); event(start); event(text); event({ type: 'turn.failed', error: { message: 'Synthetic failure, do not retry' } }); }
else if (scenario === 'hang' || scenario === 'ignore-abort') {
  event(thread); event(start);
  if (scenario === 'ignore-abort') process.on('SIGTERM', () => {});
  setTimeout(() => process.exit(0), scenario === 'ignore-abort' ? 1600 : 30_000);
} else if (scenario === 'oversized-event') { event(thread); event(start); event({ ...text, item: { ...text.item, text: 'x'.repeat(1_048_577) } }); }
else if (scenario === 'same-item-mutated') { event(thread); event(start); event(text); event({ ...text, item: { ...text.item, text: '{"tampered":true}' } }); event(complete); }
else {
  event(thread); event(start); event(text); event(complete);
  if (scenario === 'duplicates') { event(text); event(complete); }
}
