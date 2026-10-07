import test from 'node:test';
import assert from 'node:assert/strict';
import { promises as fs } from 'node:fs';
import { lstatSync, mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { developmentBuildPlan, developmentSigningEnvironment, signDevelopmentBundle, repositoryRoot, runDevelopmentBuild } from './build_macos_dev.mjs';
import { boundedNativeCommand, verifyMacosBundle } from './verify_macos_bundle.mjs';
import { packageReadiness } from '../sdk-bridge/scripts/package-readiness.mjs';
import { compileAttributeCollector, captureAttributeSnapshot, compareAttributeSnapshots, decodeAttributes } from './macos_xattr_proof.mjs';

async function fixture(t, { signed = true } = {}) {
  const directory = await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(), 'director-next-m5-synthetic-')));
  t.after(() => fs.rm(directory, { recursive: true, force: true }));
  const app = path.join(directory, 'Synthetic.app');
  await fs.mkdir(path.join(app, 'Contents/MacOS'), { recursive: true });
  await fs.writeFile(path.join(app, 'Contents/Info.plist'), '<plist><dict/></plist>');
  await fs.writeFile(path.join(app, 'Contents/MacOS/director-desktop-next'), Buffer.from('cffaedfe00000000', 'hex'), { mode: 0o755 });
  await packageReadiness(path.join(app, 'Contents/Resources/sdk-readiness'));
  const seal = path.join(app, 'Contents/_CodeSignature/CodeResources');
  if (signed) { await fs.mkdir(path.dirname(seal)); await fs.writeFile(seal, 'synthetic seal; not native evidence'); }
  const calls = [];
  const collector = path.join(directory, 'synthetic-collector');
  const proofDirectory = path.join(directory, 'proof');
  await fs.mkdir(proofDirectory);
  const command = (executable, args) => {
    calls.push([executable, [...args]]);
    if (executable === '/usr/bin/plutil') return { status: 0, stdout: args[1] === 'CFBundleIdentifier' ? 'app.director.next\n' : 'director-desktop-next\n', stderr: '' };
    if (executable === collector) return { status: 0, stdout: JSON.stringify({ schemaVersion: 1, ok: true,
      kind: lstatSync(args[0]).isDirectory() ? 'directory' : 'file', attributes: [] }), stderr: '' };
    if (args[0] === '--force') { mkdirSync(path.dirname(seal), { recursive: true }); writeFileSync(seal, 'synthetic seal; not native evidence'); }
    if (args[0] === '--display') return { status: 0, stdout: '', stderr: 'Signature=adhoc\nTeamIdentifier=not set\nCodeDirectory v=20500 flags=0x10002(adhoc,runtime)\n' };
    return { status: 0, stdout: '', stderr: '' };
  };
  return { app, seal, calls, command, collector, proofDirectory, runtime: { platform: 'darwin', command, collector, proofDirectory } };
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
  const result = await signDevelopmentBundle(f.app, f.runtime);
  assert.equal(result.existingAttributesPreserved, true);
  assert.equal(f.calls.some(([exe]) => exe === '/usr/bin/xattr'), false);
  assert.ok(f.calls.some(([exe, args]) => exe === f.collector && args[0] === f.app));
  assert.deepEqual(f.calls.find(([, args]) => args[0] === '--force'), ['/usr/bin/codesign', ['--force', '--sign', '-', '--options', 'runtime', f.app]]);
});

test('changed root quarantine bytes fail with sanitized before/after proof and no retry', async t => {
  const f = await fixture(t); let reads = 0, signed = false;
  await assert.rejects(signDevelopmentBundle(f.app, { ...f.runtime, command: (exe, args) => {
    if (args[0] === '--force') signed = true;
    if (exe === f.collector && args[0] === f.app) {
      reads++;
      return { status: 0, stdout: JSON.stringify({ schemaVersion: 1, ok: true, kind: 'directory', attributes: [
        { nameHex: Buffer.from('com.apple.quarantine').toString('hex'), valueHex: Buffer.from(signed ? 'private-after' : 'private-before').toString('hex') }] }), stderr: '' };
    }
    return f.command(exe, args);
  } }), /XATTR_CHANGED/);
  assert.equal(reads, 2); assert.equal(f.calls.filter(([, args]) => args[0] === '--force').length, 1);
  assert.equal(f.calls.some(([, args]) => args[0] === '--verify'), false);
  const before = await fs.readFile(path.join(f.proofDirectory, 'xattrs-before.json'), 'utf8');
  const after = await fs.readFile(path.join(f.proofDirectory, 'xattrs-after.json'), 'utf8');
  assert.equal(before.includes('private-before'), false); assert.equal(after.includes('private-after'), false);
  const decision = JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'xattrs-decision.json'), 'utf8'));
  assert.equal(decision.differences[0].kind, 'attribute_value_changed'); assert.equal(decision.differences[0].path, '.');
});

test('signing failure stops before verification or any retry', async t => {
  const f = await fixture(t); let signs = 0;
  await assert.rejects(signDevelopmentBundle(f.app, { ...f.runtime, command: (exe, args) => args[0] === '--force'
    ? (++signs, { status: 1, stdout: '', stderr: '' }) : f.command(exe, args) }), /ADHOC_SIGNATURE_FAILED/);
  assert.equal(signs, 1); assert.equal(f.calls.some(([, args]) => args[0] === '--verify'), false);
});

test('known core helper is signed first and verified separately, never deep-signed', async t => {
  const f = await fixture(t);
  const helper = path.join(f.app, 'Contents/MacOS/director-next-core');
  await fs.writeFile(helper, Buffer.from('cffaedfe00000000', 'hex'), { mode: 0o755 });
  const result = await signDevelopmentBundle(f.app, f.runtime);
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

test('fresh attribute-free signing nodes are explicitly recorded', async t => {
  const f = await fixture(t, { signed: false });
  const result = await signDevelopmentBundle(f.app, f.runtime);
  assert.deepEqual(result.attributeProof.allowedNewSignatureNodes, ['Contents/_CodeSignature', 'Contents/_CodeSignature/CodeResources']);
  assert.equal(JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'xattrs-before.json'), 'utf8')).nodes.some(node => node.path === '.'), true);
});

test('collection command failures, timeouts and overflow are distinct from actual diffs', async t => {
  for (const [kind, expected] of [['command_exit', 'COLLECTION_FAILED'], ['timeout', 'COLLECTION_TIMEOUT'], ['output_limit', 'COLLECTION_OUTPUT_LIMIT']]) {
    const f = await fixture(t); let signed = false;
    await assert.rejects(signDevelopmentBundle(f.app, { ...f.runtime, command: (exe, args) => {
      if (args[0] === '--force') signed = true;
      return exe === f.collector && signed ? { status: -1, stdout: '', stderr: 'not persisted', failure: { kind } } : f.command(exe, args);
    } }), new RegExp(expected));
    const after = JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'xattrs-after.json'), 'utf8'));
    assert.equal(after.complete, false); assert.equal(after.failure.kind, kind);
    assert.equal(JSON.stringify(after).includes('not persisted'), false);
    assert.equal(f.calls.some(([, args]) => args[0] === '--verify'), false);
  }
});

test('before collection failure persists evidence without signing or invented after snapshot', async t => {
  const f = await fixture(t);
  await assert.rejects(signDevelopmentBundle(f.app, { ...f.runtime, command: (exe, args) => exe === f.collector
    ? { status: 1, stdout: JSON.stringify({ schemaVersion: 1, ok: false, error: 'PERMISSION_DENIED' }), stderr: '' } : f.command(exe, args) }), /COLLECTION_FAILED/);
  assert.equal(f.calls.some(([, args]) => args[0] === '--force'), false);
  const decision = JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'xattrs-decision.json'), 'utf8'));
  assert.equal(decision.signingAttempted, false); assert.equal(decision.afterSnapshotAvailable, false);
  await assert.rejects(fs.stat(path.join(f.proofDirectory, 'xattrs-after.json')), /ENOENT/);
});

test('attribute ordering is canonical and binary values after NUL remain significant', () => {
  const a = { nameHex: Buffer.from('user.a').toString('hex'), valueHex: '0001ff0a' };
  const b = { nameHex: Buffer.from('user.b').toString('hex'), valueHex: '' };
  assert.deepEqual(decodeAttributes({ attributes: [a, b] }), decodeAttributes({ attributes: [b, a] }));
  assert.notEqual(decodeAttributes({ attributes: [a] })[0].sha256, decodeAttributes({ attributes: [{ ...a, valueHex: '0002ff0a' }] })[0].sha256);
});

test('directory attributes and every existing key change block; no com.apple exemption', () => {
  const attribute = decodeAttributes({ attributes: [{ nameHex: Buffer.from('com.apple.provenance').toString('hex'), valueHex: '00' }] })[0];
  const before = { complete: true, nodes: [{ path: '.', kind: 'directory', attributes: [] }, { path: 'Contents', kind: 'directory', attributes: [attribute] }] };
  for (const attrs of [[], [{ ...attribute, sha256: '0'.repeat(64) }], [attribute, { ...attribute, nameHex: '61' }]]) {
    const after = { complete: true, nodes: [{ ...before.nodes[1], attributes: attrs }, before.nodes[0]] };
    assert.equal(compareAttributeSnapshots(before, after).allowed, false);
  }
  assert.equal(compareAttributeSnapshots(before, { complete: true, nodes: [...before.nodes].reverse() }).allowed, true);
});

test('new signature nodes carrying any attributes and arbitrary new nodes remain blocked', () => {
  const before = { complete: true, nodes: [{ path: '.', kind: 'directory', attributes: [] }] };
  const seal = { path: 'Contents/_CodeSignature/CodeResources', kind: 'file', attributes: [] };
  const attrs = decodeAttributes({ attributes: [{ nameHex: Buffer.from('com.apple.quarantine').toString('hex'), valueHex: '' }] });
  assert.equal(compareAttributeSnapshots(before, { complete: true, nodes: [...before.nodes, { ...seal, attributes: attrs }] }).allowed, false);
  assert.equal(compareAttributeSnapshots(before, { complete: true, nodes: [...before.nodes, { ...seal, path: 'unexpected' }] }).allowed, false);
});

test('malformed, duplicate and oversized collector records never become partial success', () => {
  for (const attributes of [[{ nameHex: '00', valueHex: '' }], [{ nameHex: '61', valueHex: '0' }],
    [{ nameHex: '61', valueHex: '' }, { nameHex: '61', valueHex: '' }], [{ nameHex: '61', valueHex: '00'.repeat(65537) }]]) {
    assert.throws(() => decodeAttributes({ attributes }), /COLLECTOR_PROTOCOL/);
  }
});

test('real native collector protocol integrates with Node hashing and root/directory inventory', { skip: !['linux', 'darwin'].includes(process.platform) }, async t => {
  const f = await fixture(t);
  const collector = await compileAttributeCollector(f.proofDirectory, runDevelopmentBuild);
  const snapshot = await captureAttributeSnapshot(await fs.realpath(f.app), { collector, command: boundedNativeCommand });
  assert.equal(snapshot.complete, true, JSON.stringify(snapshot.failure));
  assert.equal(snapshot.nodes.find(node => node.path === '.').kind, 'directory');
  assert.equal(snapshot.nodes.find(node => node.path === 'Contents/MacOS/director-desktop-next').kind, 'file');
  assert.equal(JSON.stringify(snapshot).includes('valueHex'), false);
  assert.equal(JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'collector.json'), 'utf8')).compiled, true);
});

test('collector compilation timeout stops linker-like descendants and persists failure proof', { skip: process.platform === 'win32' }, async t => {
  const f = await fixture(t);
  const marker = path.join(f.proofDirectory, 'linker-survived');
  const child = 'require("node:fs").writeFileSync(process.argv[1]+".ready","ready");setTimeout(()=>require("node:fs").writeFileSync(process.argv[1],"unexpected"),800);setInterval(()=>{},1000)';
  const parent = 'process.on("SIGTERM",()=>{});require("node:child_process").spawn(process.execPath,["-e",process.argv[1],process.argv[2]],{stdio:"ignore"});setInterval(()=>{},1000)';
  await assert.rejects(compileAttributeCollector(f.proofDirectory, async plan => {
    assert.equal(plan.executable, '/usr/bin/cc'); assert.equal(plan.options.timeout, 30000);
    assert.deepEqual(Object.keys(plan.options.env).sort(), ['LANG', 'LC_ALL', 'PATH']);
    return runDevelopmentBuild({ ...plan, executable: process.execPath, args: ['-e', parent, child, marker],
      options: { ...plan.options, timeout: 250 } });
  }), /COLLECTOR_COMPILE_FAILED/);
  assert.equal(await fs.readFile(marker + '.ready', 'utf8'), 'ready');
  await new Promise(resolve => setTimeout(resolve, 900));
  assert.equal(await fs.stat(marker).then(() => true, () => false), false);
  const proof = JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'collector.json'), 'utf8'));
  assert.equal(proof.compiled, false); assert.equal(proof.failureKind, 'timeout');
});

function provenanceCommand(f, transform = value => value) {
  return (exe, args) => {
    if (exe === f.collector) {
      const data = { schemaVersion: 1, ok: true, kind: lstatSync(args[0]).isDirectory() ? 'directory' : 'file', attributes: [
        { nameHex: Buffer.from('com.apple.provenance').toString('hex'), valueHex: '0100ff2a' }] };
      return { status: 0, stdout: JSON.stringify(transform(data, args[0])), stderr: '' };
    }
    return f.command(exe, args);
  };
}

test('matching new signature provenance is accepted only with full artifact and fresh-attribute proof', async t => {
  const f = await fixture(t, { signed: false });
  const report = await signDevelopmentBundle(f.app, { ...f.runtime, command: provenanceCommand(f) });
  assert.equal(report.existingAttributesPreserved, true); assert.equal(report.newSignatureProvenanceMatched, true);
  assert.equal(report.extendedAttributesPreserved, undefined);
  const verdict = JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'artifact-verdict.json'), 'utf8'));
  assert.equal(verdict.accepted, true); assert.equal(verdict.signatureAndResourcesVerified, true);
  assert.equal(verdict.finalAttributesUnchanged, true); assert.equal(verdict.notarizationVerified, false);
  assert.equal(verdict.gatekeeperAssessmentPerformed, false); assert.equal(verdict.appLaunched, false);
  assert.match(verdict.fileInventorySha256, /^[0-9a-f]{64}$/); assert.match(verdict.finalSha256, /^[0-9a-f]{64}$/);
});

test('matching metadata cannot override strict signature failure or create overall acceptance', async t => {
  const f = await fixture(t, { signed: false }); const safe = provenanceCommand(f);
  await assert.rejects(signDevelopmentBundle(f.app, { ...f.runtime, command: (exe, args) => args[0] === '--verify'
    ? { status: 1, stdout: '', stderr: '' } : safe(exe, args) }), /STRICT_SIGNATURE_FAILED/);
  const comparison = JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'xattrs-decision.json'), 'utf8'));
  assert.equal(comparison.allowed, true); assert.equal(comparison.scope, 'attribute_comparison_only');
  const verdict = JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'artifact-verdict.json'), 'utf8'));
  assert.equal(verdict.accepted, false); assert.equal(verdict.error, 'BUNDLE_STRICT_SIGNATURE_FAILED');
});

test('post-verification attribute change blocks accepted provenance and leaves failure evidence', async t => {
  const f = await fixture(t, { signed: false }); let verified = false;
  const safe = provenanceCommand(f, (data, item) => {
    if (verified && item === f.app) data.attributes[0].valueHex = '0200ff2a';
    return data;
  });
  await assert.rejects(signDevelopmentBundle(f.app, { ...f.runtime, command: (exe, args) => {
    if (args[0] === '--verify') verified = true;
    return safe(exe, args);
  } }), /XATTR_CHANGED/);
  const verdict = JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'artifact-verdict.json'), 'utf8'));
  assert.equal(verdict.accepted, false); assert.match(verdict.finalSha256, /^[0-9a-f]{64}$/);
});

test('post-signature file mutation cannot reuse the earlier verified artifact inventory', async t => {
  const f = await fixture(t, { signed: false }); let verified = false, changed = false;
  const safe = provenanceCommand(f, data => {
    if (verified && !changed) {
      changed = true;
      writeFileSync(path.join(f.app, 'Contents/Resources/sdk-readiness/src/health.mjs'), 'changed after verification');
    }
    return data;
  });
  await assert.rejects(signDevelopmentBundle(f.app, { ...f.runtime, command: (exe, args) => {
    if (args[0] === '--verify') verified = true;
    return safe(exe, args);
  } }), /ARTIFACT_CHANGED_AFTER_VERIFICATION/);
  const verdict = JSON.parse(await fs.readFile(path.join(f.proofDirectory, 'artifact-verdict.json'), 'utf8'));
  assert.equal(verdict.accepted, false);
});
