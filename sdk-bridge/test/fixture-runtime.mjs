import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const source = fileURLToPath(new URL('./fixtures/fake-codex.mjs', import.meta.url));
export const SCENARIOS = Object.freeze(['happy', 'duplicates', 'out-of-order', 'malformed-json', 'early-complete', 'changed-thread', 'policy-violation', 'failed-after-text', 'hang', 'ignore-abort', 'invalid-text-json', 'oversized-event', 'same-item-mutated']);
const quote = text => "'" + text.replaceAll("'", "'\\''") + "'";
export function offlineFixture(scenario) {
  if (!SCENARIOS.includes(scenario)) throw new Error('Unknown offline fixture scenario');
  if (process.platform === 'win32') throw new Error('Synthetic executable launcher is POSIX only; Windows native tests are pending');
  return Object.freeze({
    async create(directory) {
      const isolated = path.join(directory, 'synthetic-runtime');
      await fs.mkdir(isolated, { recursive: true, mode: 0o700 });
      const home = path.join(isolated, 'home');
      const work = path.join(isolated, 'work');
      await fs.mkdir(home, { mode: 0o700 }); await fs.mkdir(work, { mode: 0o700 });
      const script = path.join(isolated, 'fake-codex.mjs');
      await fs.copyFile(source, script);
      const executable = path.join(isolated, 'fake-codex');
      await fs.writeFile(executable, `#!/bin/sh\nexec ${quote(process.execPath)} ${quote(script)} "$@"\n`, { mode: 0o700 });
      return { executable, workingDirectory: work, env: { HOME: home, CODEX_HOME: home, TMPDIR: isolated, DIRECTOR_FIXTURE_SCENARIO: scenario, DIRECTOR_FIXTURE_CAPTURE: path.join(directory, 'fixture-invocation.json') } };
    },
  });
}
