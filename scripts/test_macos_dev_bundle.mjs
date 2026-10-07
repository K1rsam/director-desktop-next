import test from 'node:test';
import assert from 'node:assert/strict';
import { promises as fs } from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { developmentBuildPlan, developmentSigningEnvironment, signDevelopmentBundle, repositoryRoot, runDevelopmentBuild } from './build_macos_dev.mjs';
import { boundedNativeCommand, verifyMacosBundle } from './verify_macos_bundle.mjs';
import { packageReadiness } from '../sdk-bridge/scripts/package-readiness.mjs';

async function fixture(t, { signed = true } = {}) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'director-next-m5-synthetic-'));
  t.after(() => fs.rm(directory, { recursive: true, force: true }));
  const app = path.join(directory, 'Synthetic.app');
  await fs.mkdir(path.join(app, 'Contents/MacOS'), { recursive: true });
  await fs.writeFile(path.join(app, 'Contents/Info.plist'), '<plist><dict/></plist>');
  await fs.writeFile(path.join(app, 'Contents/MacOS/director-desktop-next'), Buffer.from('cffaedfe00000000', 'hex'), { mode: 0o755 });
  await packageReadiness(path.join(app, 'Contents/Resources/sdk-readiness'));
  const seal = path.join(app, 'Contents/_CodeSignature/CodeResources');
  if (signed) { await fs.mkdir(path.dirname(seal)); await fs.writeFile(seal, 'synthetic seal; not native evidence'); }
  const calls = [];
  const command = (executable, args) => {
    calls.push([executable, [...args]]);
    if (executable === '/usr/bin/plutil') return { status: 0, stdout: args[1] === 'CFBundleIdentifier' ? 'app.director.next\n' : 'director-desktop-next\n', stderr: '' };
    if (executable === '/usr/bin/xattr') return { status: 0, stdout: 'synthetic-preserved-attribute\n', stderr: '' };
    if (args[0] === '--display') return { status: 0, stdout: '', stderr: 'Signature=adhoc\nTeamIdentifier=not set\nCodeDirectory v=20500 flags=0x10002(adhoc,runtime)\n' };
    return { status: 0, stdout: '', stderr: '' };
  };
  return { app, seal, calls, command };
}

test('development build is Mac-only, architecture-bound and rejects argument overrides', () => {
  assert.throws(() => developmentBuildPlan({ platform: 'linux' }), /REQUIRES_MACOS/);
  assert.throws(() => developmentBuildPlan({ platform: 'darwin', architecture: 'ppc' }), /ARCHITECTURE/);
  assert.throws(() => developmentBuildPlan({ platform: 'darwin', args: ['--config', 'other'] }), /NO_EXTRA_ARGUMENTS/);
  const plan = developmentBuildPlan({ platform: 'darwin', architecture: 'arm64', environment: {}, targetDirectory: path.join(repositoryRoot, '.local/macos-dev-target-synthetic') });
  assert.ok(plan.args.includes('--no-sign')); assert.ok(plan.args.includes('--ci'));
  assert.ok(plan.args.includes('desktop,custom-protocol')); assert.ok(plan.args.includes('aarch64-apple-darwin'));
  assert.match(plan.app, /\.local\/macos-dev-target-synthetic\/aarch64-apple-darwin\/release\/bundle\/macos\//);
  assert.equal(plan.options.shell, false); assert.equal(plan.options.stdio, 'inherit');
  assert.equal(plan.options.timeout, 30 * 60 * 1000); assert.equal(plan.options.killSignal, 'SIGKILL');
  assert.throws(() => developmentBuildPlan({ platform: 'darwin', targetDirectory: '/unrelated/path' }), /TARGET_INVALID/);
});

test('child environment removes all Apple credentials without changing caller environment', () => {
  const source = { PATH: '/synthetic', APPLE_SIGNING_IDENTITY: 'not-used', APPLE_CERTIFICATE: 'not-used',
    APPLE_CERTIFICATE_PASSWORD: 'not-used', APPLE_ID: 'not-used', APPLE_PASSWORD: 'not-used',
    APPLE_TEAM_ID: 'not-used', APPLE_API_KEY: 'not-used', APPLE_API_ISSUER: 'not-used',
    APPLE_API_KEY_PATH: 'not-used', APPLE_PROVIDER_SHORT_NAME: 'not-used', apple_future_credential: 'not-used' };
  assert.deepEqual(developmentSigningEnvironment(source), { PATH: '/synthetic' });
  assert.equal(source.APPLE_CERTIFICATE, 'not-used');
});

test('native command collector keeps success stderr and bounds captured output', () => {
  const success = boundedNativeCommand(process.execPath, ['-e', 'process.stderr.write("synthetic metadata")']);
  assert.equal(success.status, 0); assert.equal(success.stderr, 'synthetic metadata');
  const flood = boundedNativeCommand(process.execPath, ['-e', 'process.stdout.write("x".repeat(1048576))']);
  assert.equal(flood.status, -1); assert.ok(flood.stdout.length < 1048576);
});

test('native timeout returns even when the exact child ignores SIGTERM', () => {
  const start = Date.now();
  const result = boundedNativeCommand(process.execPath, ['-e', 'process.on("SIGTERM",()=>{});setInterval(()=>{},1000)'], 150);
  assert.equal(result.status, -1); assert.ok(Date.now() - start < 3000);
});

test('build timeout stops its owned process group including compiler-like descendants', { skip: process.platform === 'win32' }, async t => {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'director-next-m5-process-'));
  t.after(() => fs.rm(directory, { recursive: true, force: true }));
  const marker = path.join(directory, 'descendant-survived');
  const grandchild = 'require("node:fs").writeFileSync(process.argv[1]+".ready","ready");setTimeout(()=>require("node:fs").writeFileSync(process.argv[1],"unexpected"),800);setInterval(()=>{},1000)';
  const parent = 'process.on("SIGTERM",()=>{});require("node:child_process").spawn(process.execPath,["-e",process.argv[1],process.argv[2]],{stdio:"ignore"});setInterval(()=>{},1000)';
  const start = Date.now();
  const result = await runDevelopmentBuild({ executable: process.execPath, args: ['-e', parent, grandchild, marker],
    options: { cwd: directory, env: {}, shell: false, stdio: 'ignore', timeout: 250, killSignal: 'SIGKILL' } });
  assert.equal(result.timedOut, true); assert.equal(result.stopError, false); assert.equal(result.unknown, false);
  assert.ok(Date.now() - start < 3000);
  assert.equal(await fs.readFile(marker + '.ready', 'utf8'), 'ready');
  await new Promise(resolve => setTimeout(resolve, 900));
  assert.equal(await fs.stat(marker).then(() => true, () => false), false);
});

test('synthetic strict verifier records complete inventory and genuine SDK bytes, without launching', async t => {
  const f = await fixture(t);
  const result = await verifyMacosBundle(f.app, { platform: 'darwin', command: f.command });
  assert.equal(result.signature.identity, 'adhoc'); assert.equal(result.sdk.verifiedResourceFiles, 7);
  assert.equal(result.appLaunched, false); assert.equal(result.generationCalls, 0);
  assert.equal(result.notarizationVerified, false); assert.equal(result.gatekeeperAssessmentPerformed, false);
  assert.ok(result.files.some(item => item.path === 'Contents/_CodeSignature/CodeResources'));
  assert.deepEqual(f.calls.filter(([exe]) => exe.endsWith('codesign')).map(([, args]) => args.slice(0, -1)),
    [['--verify', '--strict', '--verbose=2'], ['--display', '--verbose=4']]);
});

test('missing resource seal fails even when executable or SDK resources exist', async t => {
  const f = await fixture(t, { signed: false });
  await assert.rejects(verifyMacosBundle(f.app, { platform: 'darwin', command: f.command }), /REQUIRED_FILE_MISSING/);
  assert.equal(f.calls.some(([exe]) => exe.endsWith('codesign')), false);
});

test('SDK resource tampering is rejected before signature acceptance', async t => {
  const f = await fixture(t);
  await fs.appendFile(path.join(f.app, 'Contents/Resources/sdk-readiness/src/health.mjs'), '\n// changed');
  await assert.rejects(verifyMacosBundle(f.app, { platform: 'darwin', command: f.command }), /SDK_RESOURCE_MISMATCH/);
});

test('unexpected nested native binary requires a separate signing plan', async t => {
  const f = await fixture(t);
  await fs.writeFile(path.join(f.app, 'Contents/Resources/nested'), Buffer.from('cffaedfe00000000', 'hex'));
  await assert.rejects(verifyMacosBundle(f.app, { platform: 'darwin', command: f.command }), /NATIVE_LAYOUT_UNSUPPORTED/);
});

test('bundle symlinks and wrong app identity are rejected', async t => {
  const f = await fixture(t);
  await assert.rejects(verifyMacosBundle(f.app, { platform: 'darwin', command: () => ({ status: 0, stdout: 'old-app', stderr: '' }) }), /IDENTITY_MISMATCH/);
  await fs.symlink(path.join(f.app, 'Contents/Info.plist'), path.join(f.app, 'Contents/link'));
  await assert.rejects(verifyMacosBundle(f.app, { platform: 'darwin', command: f.command }), /INVENTORY_UNSUPPORTED/);
});

test('strict signature failure and unexpected certificate signature are not accepted', async t => {
  const f = await fixture(t);
  await assert.rejects(verifyMacosBundle(f.app, { platform: 'darwin', command: (exe, args) => args[0] === '--verify'
    ? { status: 1, stdout: '', stderr: 'synthetic failure' } : f.command(exe, args) }), /STRICT_SIGNATURE_FAILED/);
  await assert.rejects(verifyMacosBundle(f.app, { platform: 'darwin', command: (exe, args) => args[0] === '--display'
    ? { status: 0, stdout: '', stderr: 'Signature=adhoc\nAuthority=unexpected\n' } : f.command(exe, args) }), /NOT_ADHOC/);
});

test('development signing uses fixed ad-hoc argv and preserves xattrs', async t => {
  const f = await fixture(t);
  const result = await signDevelopmentBundle(f.app, { platform: 'darwin', command: f.command });
  assert.equal(result.extendedAttributesPreserved, true);
  assert.deepEqual(f.calls.filter(([exe]) => exe.endsWith('xattr')).map(([, args]) => args), [['-lr', f.app], ['-lr', f.app]]);
  assert.deepEqual(f.calls.find(([, args]) => args[0] === '--force'), ['/usr/bin/codesign', ['--force', '--sign', '-', '--options', 'runtime', f.app]]);
});

test('changed xattrs fail without restoration, retry or security alteration', async t => {
  const f = await fixture(t); let reads = 0;
  await assert.rejects(signDevelopmentBundle(f.app, { platform: 'darwin', command: (exe, args) => exe.endsWith('xattr')
    ? { status: 0, stdout: String(++reads), stderr: '' } : f.command(exe, args) }), /XATTR_CHANGED/);
  assert.equal(reads, 2); assert.equal(f.calls.filter(([, args]) => args[0] === '--force').length, 1);
  assert.equal(f.calls.some(([, args]) => args[0] === '--verify'), false);
});

test('signing failure stops before verification or any retry', async t => {
  const f = await fixture(t); let signs = 0;
  await assert.rejects(signDevelopmentBundle(f.app, { platform: 'darwin', command: (exe, args) => args[0] === '--force'
    ? (++signs, { status: 1, stdout: '', stderr: '' }) : f.command(exe, args) }), /ADHOC_SIGNATURE_FAILED/);
  assert.equal(signs, 1); assert.equal(f.calls.some(([, args]) => args[0] === '--verify'), false);
});

test('known core helper is signed first and verified separately, never deep-signed', async t => {
  const f = await fixture(t);
  const helper = path.join(f.app, 'Contents/MacOS/director-next-core');
  await fs.writeFile(helper, Buffer.from('cffaedfe00000000', 'hex'), { mode: 0o755 });
  const result = await signDevelopmentBundle(f.app, { platform: 'darwin', command: f.command });
  assert.deepEqual(f.calls.filter(([, args]) => args[0] === '--force').map(([, args]) => args.at(-1)), [helper, f.app]);
  assert.deepEqual(f.calls.filter(([, args]) => args[0] === '--verify').map(([, args]) => args.at(-1)), [f.app, helper]);
  assert.equal(result.signature.verifiedNativeFiles.length, 2);
  assert.equal(f.calls.some(([, args]) => args.includes('--deep')), false);
});

test('ad-hoc signatures without hardened runtime do not pass', async t => {
  const f = await fixture(t);
  await assert.rejects(verifyMacosBundle(f.app, { platform: 'darwin', command: (exe, args) => args[0] === '--display'
    ? { status: 0, stdout: '', stderr: 'Signature=adhoc\nCodeDirectory flags=0x2(adhoc)\n' } : f.command(exe, args) }), /HARDENED_RUNTIME_MISSING/);
});
