// Test-only preload: proves genuine SDK import/health do not spawn the Codex
// runtime or perform network I/O. No production entrypoint loads this file.
import childProcess from 'node:child_process';
import net from 'node:net';
import http from 'node:http';
import https from 'node:https';
import http2 from 'node:http2';
import { syncBuiltinESMExports } from 'node:module';
import { writeFileSync } from 'node:fs';
let attempts = 0;
const deny = () => { attempts++; throw new Error('HEALTH_FORBIDDEN_EFFECT'); };
for (const key of ['spawn', 'spawnSync', 'exec', 'execSync', 'execFile', 'execFileSync', 'fork']) childProcess[key] = deny;
net.Socket.prototype.connect = deny;
for (const module of [http, https]) for (const key of ['request', 'get']) module[key] = deny;
http2.connect = deny;
globalThis.fetch = deny;
syncBuiltinESMExports();
process.on('exit', () => writeFileSync(process.env.DIRECTOR_HEALTH_AUDIT,
  JSON.stringify({ forbiddenEffectAttempts: attempts, environmentKeys: Object.keys(process.env).sort() })));
