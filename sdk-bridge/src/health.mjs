// Production readiness entrypoint. Import the genuine pinned SDK, but never
// construct Codex, inspect an account, create/resume a thread, or run a turn.
import { Codex } from '@openai/codex-sdk';
import { OfficialSdkBridge, PROTOCOL_VERSION, SDK_VERSION } from './bridge.mjs';

if (typeof Codex !== 'function') throw new Error('SDK_IMPORT_INVALID');
const args = process.argv.slice(2);
if (args.length !== 2 || args[0] !== '--state-dir' || !args[1]) {
  throw new Error('HEALTH_ARGUMENTS: only --state-dir is allowed');
}
const bridge = new OfficialSdkBridge({ stateDirectory: args[1] });
await bridge.initialize();
const methods = ['hello', 'inspect', 'shutdown'];
let count = 0, bytes = 0, buffered = Buffer.alloc(0), closing = false;
let chain = Promise.resolve();
const deadline = setTimeout(() => void finish(1), 6000);
async function finish(code) {
  if (closing) return;
  closing = true;
  clearTimeout(deadline);
  process.stdin.destroy();
  try { await bridge.disconnect('HEALTH_CLOSED'); } catch { code = 1; }
  // A second deadline also bounds a broken stdout consumer during shutdown.
  const exitDeadline = setTimeout(() => process.exit(code), 100);
  process.stdout.end(() => { clearTimeout(exitDeadline); process.exit(code); });
}
process.on('SIGTERM', () => void finish(1));
process.on('SIGINT', () => void finish(1));
process.stdout.on('error', () => void finish(1));
process.stdin.on('error', () => void finish(1));
async function handle(line) {
  if (closing) return;
  try {
    const request = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(line));
    if (request.method !== methods[count] || request.attemptId !== null
      || request.protocolVersion !== PROTOCOL_VERSION
      || request.requestId !== `health_${count + 1}`
      || request.params !== null
      || Object.keys(request).sort().join(',') !== 'attemptId,method,params,protocolVersion,requestId') {
      throw new Error('HEALTH_CONTROL_ONLY');
    }
    // Only these three fixed control requests can reach the existing bridge.
    const response = await bridge.handle(request);
    count++;
    response.health = {
      sdkImported: true, sdkPackage: '@openai/codex-sdk', sdkVersion: SDK_VERSION,
      nodeVersion: process.version, allowedMethods: methods, controlRequests: count,
      executeCalls: 0, accountChecked: false, networkTested: false, generationEnabled: false,
    };
    const output = JSON.stringify(response) + '\n';
    if (Buffer.byteLength(output) > 16384) throw new Error('HEALTH_OUTPUT_LIMIT');
    await new Promise((resolve, reject) => process.stdout.write(output, error => error ? reject(error) : resolve()));
    if (request.method === 'shutdown') await finish(0);
  } catch { await finish(1); }
}
process.stdin.on('data', chunk => {
  bytes += chunk.length;
  if (closing || bytes > 8192) { void finish(1); return; }
  buffered = Buffer.concat([buffered, chunk]);
  let end;
  while ((end = buffered.indexOf(10)) !== -1) {
    const line = buffered.subarray(0, end);
    buffered = buffered.subarray(end + 1);
    chain = chain.then(() => handle(line));
  }
});
process.stdin.on('end', () => void chain.then(() => finish(count === 3 && !buffered.length ? 0 : 1)));
