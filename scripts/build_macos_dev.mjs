// Explicit local-development bundle signing. No certificate, account, keychain
// import, notarization credentials, or OS security changes are requested here.
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';
import { promises as fs } from 'node:fs';
import { createHash } from 'node:crypto';
import { inspectMacosBundle, inventoryBundle, boundedNativeCommand, verifyMacosBundle } from './verify_macos_bundle.mjs';
import { compileAttributeCollector, captureAttributeSnapshot, compareAttributeSnapshots, writeProof } from './macos_xattr_proof.mjs';

export const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

export function developmentSigningEnvironment(source) {
  // Tauri gives environment credentials precedence over configuration. Remove
  // the entire Apple namespace from this child only; never print its values.
  const environment = Object.fromEntries(Object.entries(source)
    .filter(([key]) => !key.toUpperCase().startsWith('APPLE_')));
  return environment;
}

export function developmentBuildPlan({ platform = process.platform, architecture = process.arch, args = [], environment = process.env, targetDirectory } = {}) {
  if (platform !== 'darwin') throw new Error('MAC_DEV_BUILD_REQUIRES_MACOS');
  if (!['arm64', 'x64'].includes(architecture)) throw new Error('MAC_DEV_BUILD_ARCHITECTURE_UNSUPPORTED');
  if (args.length) throw new Error('MAC_DEV_BUILD_ACCEPTS_NO_EXTRA_ARGUMENTS');
  if (!targetDirectory || path.dirname(targetDirectory) !== path.join(repositoryRoot, '.local')
    || !path.basename(targetDirectory).startsWith('macos-dev-target-')) throw new Error('MAC_DEV_TARGET_INVALID');
  const desktop = path.join(repositoryRoot, 'apps/desktop');
  const target = architecture === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
  return {
    app: path.join(targetDirectory, target, 'release/bundle/macos/导演工作台 Next.app'),
    executable: process.execPath,
    args: [path.join(desktop, 'node_modules/@tauri-apps/cli/tauri.js'),
      'build', '--features', 'desktop,custom-protocol', '--bundles', 'app',
      '--target', target, '--no-sign', '--ci', '--', '--target-dir', targetDirectory],
    options: { cwd: desktop, env: { ...developmentSigningEnvironment(environment), CARGO_TARGET_DIR: targetDirectory },
      shell: false, stdio: 'inherit', timeout: 30 * 60 * 1000, killSignal: 'SIGKILL' },
  };
}

export async function runDevelopmentBuild(plan) {
  if (process.platform === 'win32') throw new Error('MAC_DEV_BUILD_PROCESS_GROUP_UNAVAILABLE');
  return new Promise(resolve => {
    let finished = false, reason = null, timer, reapTimer, stopError = false;
    const { timeout, killSignal: _killSignal, ...options } = plan.options;
    // A new POSIX session/process group belongs only to this explicit build.
    const child = spawn(plan.executable, plan.args, { ...options, detached: true });
    const signalGroup = () => {
      if (Number.isInteger(child.pid) && child.pid > 0) {
        try { process.kill(-child.pid, 'SIGKILL'); }
        catch (error) { if (error.code !== 'ESRCH') stopError = true; }
      }
    };
    const finish = result => {
      if (finished) return;
      finished = true;
      clearTimeout(timer); clearTimeout(reapTimer);
      process.removeListener('SIGINT', interrupted); process.removeListener('SIGTERM', interrupted);
      child.unref();
      resolve({ ...result, timedOut: reason === 'timeout', interrupted: reason === 'interrupt', stopError });
    };
    const stop = why => {
      if (finished || reason) return;
      reason = why;
      signalGroup();
      reapTimer = setTimeout(() => finish({ status: null, signal: null, unknown: true }), 2000);
    };
    const interrupted = () => stop('interrupt');
    process.on('SIGINT', interrupted); process.on('SIGTERM', interrupted);
    child.once('error', () => { signalGroup(); finish({ status: null, error: true }); });
    child.once('exit', (status, signal) => {
      if (status !== 0 || signal || reason) signalGroup();
      finish({ status, signal, unknown: false });
    });
    timer = setTimeout(() => stop('timeout'), timeout);
  });
}

export async function signDevelopmentBundle(app, { platform = process.platform, command = boundedNativeCommand, collector, proofDirectory } = {}) {
  // Tauri's own signing branch clears xattrs. --no-sign above bypasses it;
  // direct codesign avoids that clearing step. Structured evidence determines
  // whether existing attributes actually remained unchanged.
  const inspection = await inspectMacosBundle(app, { platform, command });
  if (!collector || !proofDirectory) throw new Error('MAC_DEV_XATTR_PROOF_REQUIRED');
  const proofRoot = await fs.realpath(proofDirectory);
  if (proofRoot === inspection.root || proofRoot.startsWith(inspection.root + path.sep)) throw new Error('MAC_DEV_XATTR_PROOF_INSIDE_APP');
  const before = await captureAttributeSnapshot(inspection.root, { collector, command });
  const beforeSha256 = await writeProof(proofRoot, 'xattrs-before.json', before);
  if (!before.complete) {
    await writeProof(proofRoot, 'xattrs-decision.json', { schema: 'macos-xattr-decision/v1', allowed: false,
      scope: 'attribute_comparison_only', error: before.error, signingAttempted: false, beforeSha256, afterSnapshotAvailable: false });
    await writeProof(proofRoot, 'artifact-verdict.json', { schema: 'macos-development-artifact/v1', accepted: false,
      error: before.error, signingAttempted: false, beforeSha256, appLaunched: false });
    throw new Error(before.error);
  }
  const targets = inspection.nativeFiles.filter(name => name !== 'Contents/MacOS/director-desktop-next')
    .map(name => path.join(inspection.root, name));
  targets.push(inspection.root);
  let signingFailed = false;
  for (const target of targets) {
    const signed = command('/usr/bin/codesign', ['--force', '--sign', '-', '--options', 'runtime', target]);
    if (signed.status !== 0) { signingFailed = true; break; }
  }
  const after = await captureAttributeSnapshot(inspection.root, { collector, command });
  const afterSha256 = await writeProof(proofRoot, 'xattrs-after.json', after);
  const decision = compareAttributeSnapshots(before, after, { allowMatchingSignatureProvenance: true });
  await writeProof(proofRoot, 'xattrs-decision.json', { schema: 'macos-xattr-decision/v1', ...decision,
    signingAttempted: true, signingSucceeded: !signingFailed, beforeSha256, afterSha256 });
  const verdict = { schema: 'macos-development-artifact/v1', scope: 'local_development_artifact_only', accepted: false,
    beforeSha256, afterSha256, signingAttempted: true, signingSucceeded: !signingFailed,
    attributeComparisonAllowed: decision.allowed, appLaunched: false, notarizationVerified: false, gatekeeperAssessmentPerformed: false };
  try {
    if (signingFailed) throw new Error('MAC_DEV_ADHOC_SIGNATURE_FAILED');
    if (!decision.allowed) throw new Error(decision.error);
    const report = await verifyMacosBundle(app, { platform, command });
    const finalAttributes = await captureAttributeSnapshot(inspection.root, { collector, command });
    const finalSha256 = await writeProof(proofRoot, 'xattrs-final.json', finalAttributes);
    verdict.finalSha256 = finalSha256;
    const finalDecision = compareAttributeSnapshots(after, finalAttributes);
    await writeProof(proofRoot, 'xattrs-final-decision.json', { schema: 'macos-xattr-decision/v1', ...finalDecision,
      afterSha256, finalSha256, scope: 'post_verification_attribute_comparison_only' });
    if (!finalDecision.allowed) throw new Error(finalDecision.error);
    const finalFiles = await inventoryBundle(inspection.root);
    if (JSON.stringify(finalFiles) !== JSON.stringify(report.files)) throw new Error('MAC_DEV_ARTIFACT_CHANGED_AFTER_VERIFICATION');
    verdict.fileInventorySha256 = createHash('sha256').update(JSON.stringify(report.files)).digest('hex');
    verdict.signatureAndResourcesVerified = true;
    verdict.finalAttributesUnchanged = true;
    verdict.existingAttributesPreserved = true;
    verdict.newSignatureProvenanceMatched = decision.newSignatureProvenanceMatched;
    verdict.accepted = true;
    await writeProof(proofRoot, 'artifact-verdict.json', verdict);
    report.existingAttributesPreserved = true;
    report.newSignatureProvenanceMatched = decision.newSignatureProvenanceMatched;
    report.attributeProof = { beforeSha256, afterSha256, finalSha256, fileInventorySha256: verdict.fileInventorySha256,
      allowedNewSignatureNodes: decision.allowedNewSignatureNodes, provenanceEvidence: decision.provenanceEvidence || null };
    return report;
  } catch (error) {
    verdict.error = typeof error.message === 'string' && /^(MAC_DEV|BUNDLE)_[A-Z_]+$/.test(error.message)
      ? error.message : 'MAC_DEV_ARTIFACT_VERIFICATION_FAILED';
    await writeProof(proofRoot, 'artifact-verdict.json', verdict);
    throw error;
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.platform !== 'darwin') throw new Error('MAC_DEV_BUILD_REQUIRES_MACOS');
    if (process.argv.length !== 2) throw new Error('MAC_DEV_BUILD_ACCEPTS_NO_EXTRA_ARGUMENTS');
    const metadata = JSON.parse(await fs.readFile(path.join(repositoryRoot, 'apps/desktop/node_modules/@tauri-apps/cli/package.json'), 'utf8'));
    if (metadata.name !== '@tauri-apps/cli' || metadata.version !== '2.12.1') throw new Error('MAC_DEV_TAURI_VERSION_MISMATCH');
    const local = path.join(repositoryRoot, '.local');
    await fs.mkdir(local, { recursive: true });
    if ((await fs.lstat(local)).isSymbolicLink()) throw new Error('MAC_DEV_TARGET_SYMLINK');
    // A brand-new target tree cannot contain old descendant symlinks that could
    // redirect Cargo output before a post-build containment check.
    const target = await fs.mkdtemp(path.join(local, 'macos-dev-target-'));
    const plan = developmentBuildPlan({ targetDirectory: target });
    const result = await runDevelopmentBuild(plan);
    if (result.error || result.signal || result.status !== 0 || result.timedOut || result.interrupted || result.stopError || result.unknown) {
      console.error('MAC_DEV_BUILD_FAILED');
      process.exitCode = Number.isInteger(result.status) && result.status > 0 ? result.status : 1;
    } else {
      if (!(await fs.realpath(plan.app)).startsWith((await fs.realpath(target)) + path.sep)) throw new Error('MAC_DEV_TARGET_ESCAPED');
      const directory = await fs.mkdtemp(path.join(repositoryRoot, '.local/macos-dev-evidence-'));
      console.log(`Signature/resource evidence directory: ${directory}`);
      const collector = await compileAttributeCollector(directory, runDevelopmentBuild);
      const report = await signDevelopmentBundle(plan.app, { collector, proofDirectory: directory });
      await fs.writeFile(path.join(directory, 'bundle.json'), JSON.stringify(report, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
      console.log(`Verified development bundle: ${plan.app}`);
      console.log(`Signature/resource evidence: ${path.join(directory, 'bundle.json')}`);
    }
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
