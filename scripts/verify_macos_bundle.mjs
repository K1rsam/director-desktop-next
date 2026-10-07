// Read-only bundle verification; the only write is the caller's fresh report.
// Does not launch the app, sign it, assess/change Gatekeeper, or read credentials.
import { createHash } from 'node:crypto';
import { createReadStream, promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { resourceFiles, sdkRoot, pinned } from '../sdk-bridge/scripts/package-readiness.mjs';

const fail = code => { throw new Error(code); };
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
async function fileHash(file) {
  const hash = createHash('sha256');
  for await (const block of createReadStream(file)) hash.update(block);
  return hash.digest('hex');
}
async function boundedFile(file, limit = 2_097_152) {
  const stat = await fs.lstat(file).catch(() => fail('BUNDLE_REQUIRED_FILE_MISSING'));
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size === 0 || stat.size > limit) fail('BUNDLE_FILE_INVALID');
  return fs.readFile(file);
}
export async function inventoryBundle(root) {
  const files = [];
  let entries = 0;
  async function visit(directory, prefix = '', depth = 0) {
    if (depth > 12) fail('BUNDLE_INVENTORY_LIMIT');
    for (const item of (await fs.readdir(directory, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
      if (++entries > 512 || item.isSymbolicLink()) fail('BUNDLE_INVENTORY_UNSUPPORTED');
      const relative = prefix + item.name;
      const full = path.join(directory, item.name);
      if (item.isDirectory()) await visit(full, relative + '/', depth + 1);
      else if (item.isFile()) {
        const stat = await fs.lstat(full);
        if (stat.size > 1_073_741_824) fail('BUNDLE_INVENTORY_LIMIT');
        files.push({ path: relative, bytes: stat.size, mode: (stat.mode & 0o7777).toString(8), sha256: await fileHash(full) });
      } else fail('BUNDLE_INVENTORY_UNSUPPORTED');
    }
  }
  await visit(root);
  return files;
}

// Tools are fixed system binaries. spawnSync bounds retained output and
// execution time; no shell, injected command, or caller-selected verifier exists.
export function boundedNativeCommand(executable, args, timeout = 30_000) {
  try {
    const output = spawnSync(executable, args, { encoding: 'utf8', timeout, killSignal: 'SIGKILL',
      maxBuffer: 128 * 1024, shell: false, stdio: ['ignore', 'pipe', 'pipe'],
      env: { PATH: '/usr/bin:/bin:/usr/sbin:/sbin', LANG: 'C', LC_ALL: 'C' } });
    return { status: output.error || output.signal ? -1 : output.status,
      stdout: output.stdout || '', stderr: output.stderr || '' };
  } catch (error) {
    return { status: Number.isInteger(error.status) ? error.status : -1,
      stdout: String(error.stdout || ''), stderr: String(error.stderr || '') };
  }
}

export async function inspectMacosBundle(app, { platform = process.platform, command = boundedNativeCommand } = {}) {
  if (platform !== 'darwin') fail('BUNDLE_VERIFY_REQUIRES_MACOS');
  if (!path.isAbsolute(app) || path.extname(app) !== '.app') fail('BUNDLE_PATH_INVALID');
  const stat = await fs.lstat(app).catch(() => fail('BUNDLE_PATH_INVALID'));
  if (!stat.isDirectory() || stat.isSymbolicLink()) fail('BUNDLE_PATH_INVALID');
  const root = await fs.realpath(app);
  const before = await inventoryBundle(root);
  const plist = path.join(root, 'Contents/Info.plist');
  await boundedFile(plist, 65_536);
  for (const [key, expected] of [['CFBundleIdentifier', 'app.director.next'], ['CFBundleExecutable', 'director-desktop-next']]) {
    const result = command('/usr/bin/plutil', ['-extract', key, 'raw', '-o', '-', plist]);
    if (result.status !== 0 || result.stdout.trim() !== expected) fail('BUNDLE_IDENTITY_MISMATCH');
  }
  const binary = before.find(item => item.path === 'Contents/MacOS/director-desktop-next');
  if (!binary || (parseInt(binary.mode, 8) & 0o111) === 0) fail('BUNDLE_EXECUTABLE_INVALID');
  if (before.some(item => item.path.startsWith('Contents/MacOS/')
    && !['Contents/MacOS/director-desktop-next', 'Contents/MacOS/director-next-core'].includes(item.path))) fail('BUNDLE_NATIVE_LAYOUT_UNSUPPORTED');
  // The repository declares these two binaries. Any further nested native code
  // needs an explicit signing plan; do not silently deep-sign it.
  const nativeFiles = [];
  const magics = new Set(['cffaedfe', 'cefaedfe', 'feedfacf', 'feedface', 'cafebabe', 'bebafeca', 'cafebabf', 'bfbafeca']);
  for (const item of before) {
    const file = await fs.open(path.join(root, item.path), 'r');
    try {
      const bytes = Buffer.alloc(4);
      const { bytesRead } = await file.read(bytes, 0, 4, 0);
      if (bytesRead === 4 && magics.has(bytes.toString('hex'))) nativeFiles.push(item.path);
    } finally { await file.close(); }
  }
  if (!nativeFiles.includes(binary.path) || nativeFiles.some(name => !['Contents/MacOS/director-desktop-next', 'Contents/MacOS/director-next-core'].includes(name))) fail('BUNDLE_NATIVE_LAYOUT_UNSUPPORTED');
  if (nativeFiles.some(name => (parseInt(before.find(item => item.path === name).mode, 8) & 0o111) === 0)) fail('BUNDLE_EXECUTABLE_INVALID');
  const resources = path.join(root, 'Contents/Resources/sdk-readiness');
  const manifest = JSON.parse(await boundedFile(path.join(resources, 'manifest.json')));
  if (manifest.schemaVersion !== 'sdk-readiness-resources/v1' || manifest.sdkPackage !== '@openai/codex-sdk'
    || manifest.sdkVersion !== pinned.version || manifest.officialRuntimeIncluded !== false || manifest.modelCalls !== 0
    || !Array.isArray(manifest.files) || manifest.files.length !== resourceFiles.length) fail('BUNDLE_SDK_MANIFEST_INVALID');
  for (const name of resourceFiles) {
    const entries = manifest.files.filter(item => item.path === name);
    const bundled = await boundedFile(path.join(resources, name));
    const expected = await boundedFile(path.join(sdkRoot, name));
    const fixed = name.endsWith('/package.json') ? pinned.package
      : name.endsWith('/dist/index.js') ? pinned.implementation : name.endsWith('/LICENSE') ? pinned.license : undefined;
    if (entries.length !== 1 || entries[0].bytes !== bundled.length || entries[0].sha256 !== sha256(bundled)
      || sha256(bundled) !== sha256(expected) || (fixed && sha256(bundled) !== fixed)) fail('BUNDLE_SDK_RESOURCE_MISMATCH');
  }
  return { root, files: before, nativeFiles };
}

export async function verifyMacosBundle(app, { platform = process.platform, command = boundedNativeCommand } = {}) {
  const { root, files: before, nativeFiles } = await inspectMacosBundle(app, { platform, command });
  const seal = await boundedFile(path.join(root, 'Contents/_CodeSignature/CodeResources'), 4_194_304);
  const targets = [root, ...nativeFiles.filter(name => name !== 'Contents/MacOS/director-desktop-next').map(name => path.join(root, name))];
  for (const target of targets) {
    const verify = command('/usr/bin/codesign', ['--verify', '--strict', '--verbose=2', target]);
    if (verify.status !== 0) fail('BUNDLE_STRICT_SIGNATURE_FAILED');
    const display = command('/usr/bin/codesign', ['--display', '--verbose=4', target]);
    // codesign writes display metadata to stderr even on success.
    const metadata = `${display.stdout}\n${display.stderr}`;
    if (display.status !== 0 || !/^Signature=adhoc\s*$/m.test(metadata) || /^Authority=/m.test(metadata)
      || /^TeamIdentifier=(?!not set\s*$).+/m.test(metadata)) fail('BUNDLE_NOT_ADHOC');
    if (!/flags=0x[0-9a-f]+\([^\n)]*\bruntime\b[^\n)]*\)/i.test(metadata)) fail('BUNDLE_HARDENED_RUNTIME_MISSING');
  }
  const after = await inventoryBundle(root);
  if (JSON.stringify(before) !== JSON.stringify(after)) fail('BUNDLE_CHANGED_DURING_VERIFY');
  return { schema: 'macos-development-bundle/v1', appPath: root, verifiedAt: new Date().toISOString(),
    signature: { strictVerification: true, identity: 'adhoc', hardenedRuntime: true, verifiedNativeFiles: nativeFiles, codeResourcesSha256: sha256(seal) },
    sdk: { version: pinned.version, verifiedResourceFiles: resourceFiles.length },
    appLaunched: false, generationCalls: 0, notarizationVerified: false, gatekeeperAssessmentPerformed: false,
    files: after };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = process.argv.slice(2);
    if (args.length !== 4 || args[0] !== '--app' || args[2] !== '--report' || !path.isAbsolute(args[3])) fail('BUNDLE_VERIFY_ARGUMENTS_INVALID');
    const report = await verifyMacosBundle(args[1]);
    const target = path.resolve(args[3]);
    const parent = await fs.realpath(path.dirname(target));
    if (parent === report.appPath || parent.startsWith(report.appPath + path.sep)) fail('BUNDLE_REPORT_MUST_BE_OUTSIDE_APP');
    await fs.writeFile(path.join(parent, path.basename(target)), JSON.stringify(report, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
    console.log(`Verified ad-hoc bundle and ${report.sdk.verifiedResourceFiles} SDK files; app/model calls: 0`);
  } catch (error) {
    console.error(error.message?.startsWith('BUNDLE_') ? error.message : 'BUNDLE_VERIFICATION_FAILED');
    process.exitCode = 1;
  }
}
