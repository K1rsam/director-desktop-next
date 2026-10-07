import { createHash } from 'node:crypto';
import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const hash = value => createHash('sha256').update(value).digest('hex');
const compare = (a, b) => a < b ? -1 : a > b ? 1 : 0;
const fail = code => { throw new Error(code); };
export const collectorSource = fileURLToPath(new URL('./macos_xattr_snapshot.c', import.meta.url));

export async function compileAttributeCollector(directory, runCompiler) {
  const target = path.join(directory, 'xattr-snapshot');
  const sourceSha256 = hash(await fs.readFile(collectorSource));
  const result = await runCompiler({ executable: '/usr/bin/cc',
    args: ['-std=c11', '-O2', '-Wall', '-Wextra', '-Werror', collectorSource, '-o', target],
    options: { cwd: directory, env: { PATH: '/usr/bin:/bin:/usr/sbin:/sbin', LANG: 'C', LC_ALL: 'C' },
      shell: false, stdio: 'ignore', timeout: 30_000, killSignal: 'SIGKILL' } });
  if (result.status !== 0 || result.error || result.signal || result.timedOut || result.interrupted || result.stopError || result.unknown) {
    await writeProof(directory, 'collector.json', { schema: 'macos-xattr-collector/v1', compiled: false, sourceSha256,
      error: 'MAC_DEV_XATTR_COLLECTOR_COMPILE_FAILED', failureKind: result.timedOut ? 'timeout'
        : result.interrupted ? 'interrupted' : result.stopError || result.unknown ? 'stop_unconfirmed' : result.error ? 'spawn' : 'command_exit',
      exitCode: Number.isInteger(result.status) ? result.status : null });
    fail('MAC_DEV_XATTR_COLLECTOR_COMPILE_FAILED');
  }
  await writeProof(directory, 'collector.json', { schema: 'macos-xattr-collector/v1',
    sourceSha256, executableSha256: hash(await fs.readFile(target)), compiled: true, compiler: '/usr/bin/cc' });
  return target;
}

export async function writeProof(directory, name, value) {
  const bytes = JSON.stringify(value, null, 2) + '\n';
  if (Buffer.byteLength(bytes) > 2_097_152) fail('MAC_DEV_XATTR_PROOF_LIMIT');
  await fs.writeFile(path.join(directory, name), bytes, { flag: 'wx', mode: 0o600 });
  return hash(bytes);
}

async function nodeInventory(root) {
  const nodes = [];
  async function visit(full, relative, depth) {
    if (depth > 12 || nodes.length >= 128) fail('MAC_DEV_XATTR_NODE_LIMIT');
    const stat = await fs.lstat(full);
    if (stat.isSymbolicLink() || (!stat.isFile() && !stat.isDirectory())) fail('MAC_DEV_XATTR_NODE_INVALID');
    nodes.push({ path: relative, kind: stat.isDirectory() ? 'directory' : 'file' });
    if (stat.isDirectory()) {
      for (const name of (await fs.readdir(full)).sort(compare)) await visit(path.join(full, name), relative === '.' ? name : relative + '/' + name, depth + 1);
    }
  }
  await visit(root, '.', 0);
  return nodes.sort((a, b) => compare(a.path, b.path));
}

export function decodeAttributes(value) {
  if (!value || !Array.isArray(value.attributes) || value.attributes.length > 64) fail('MAC_DEV_XATTR_COLLECTOR_PROTOCOL');
  const seen = new Set();
  let namesBytes = 0, valueBytes = 0;
  return value.attributes.map(item => {
    if (!item || typeof item.nameHex !== 'string' || !/^(?:[0-9a-f]{2})+$/i.test(item.nameHex)
      || item.nameHex.length > 131072 || typeof item.valueHex !== 'string'
      || !/^(?:[0-9a-f]{2})*$/i.test(item.valueHex) || item.valueHex.length > 131072) fail('MAC_DEV_XATTR_COLLECTOR_PROTOCOL');
    const nameHex = item.nameHex.toLowerCase();
    if (seen.has(nameHex) || Buffer.from(nameHex, 'hex').includes(0)) fail('MAC_DEV_XATTR_COLLECTOR_PROTOCOL');
    seen.add(nameHex);
    const bytes = Buffer.from(item.valueHex, 'hex');
    namesBytes += nameHex.length / 2 + 1; valueBytes += bytes.length;
    if (namesBytes > 65536 || namesBytes + valueBytes > 262144) fail('MAC_DEV_XATTR_COLLECTOR_PROTOCOL');
    const name = Buffer.from(nameHex, 'hex').toString('utf8');
    return { nameHex, ...(/^[A-Za-z0-9._:-]{1,255}$/.test(name) ? { name } : {}), bytes: bytes.length, sha256: hash(bytes) };
  }).sort((a, b) => compare(a.nameHex, b.nameHex));
}

export async function captureAttributeSnapshot(root, { collector, command, now = Date.now }) {
  const snapshot = { schema: 'macos-xattr-snapshot/v1', complete: false, nodes: [] };
  let current = '.', totalAttributes = 0, totalBytes = 0;
  try {
    const initial = await nodeInventory(root);
    const deadline = now() + 30_000;
    for (const node of initial) {
      current = node.path;
      const remaining = deadline - now();
      if (remaining <= 0) fail('MAC_DEV_XATTR_COLLECTION_TIMEOUT');
      const result = command(collector, [node.path === '.' ? root : path.join(root, node.path)], Math.min(5000, remaining), 786432);
      if (result.status !== 0) {
        let collectorCode;
        try { const payload = JSON.parse(result.stdout); if (payload.ok === false && /^[A-Z_]{1,80}$/.test(payload.error)) collectorCode = payload.error; } catch {}
        snapshot.failure = { path: node.path, kind: result.failure?.kind || 'command_exit',
          exitCode: Number.isInteger(result.status) ? result.status : null,
          ...(collectorCode ? { collectorCode } : {}) };
        fail(result.failure?.kind === 'timeout' ? 'MAC_DEV_XATTR_COLLECTION_TIMEOUT'
          : result.failure?.kind === 'output_limit' ? 'MAC_DEV_XATTR_COLLECTION_OUTPUT_LIMIT' : 'MAC_DEV_XATTR_COLLECTION_FAILED');
      }
      const payload = JSON.parse(result.stdout);
      if (payload.schemaVersion !== 1 || payload.ok !== true || payload.kind !== node.kind) fail('MAC_DEV_XATTR_COLLECTOR_PROTOCOL');
      const attributes = decodeAttributes(payload);
      totalAttributes += attributes.length; totalBytes += attributes.reduce((n, item) => n + item.bytes, 0);
      if (totalAttributes > 1024 || totalBytes > 4_194_304) fail('MAC_DEV_XATTR_COLLECTION_LIMIT');
      snapshot.nodes.push({ ...node, attributes });
    }
    if (JSON.stringify(initial) !== JSON.stringify(await nodeInventory(root))) fail('MAC_DEV_XATTR_TREE_CHANGED');
    snapshot.complete = true;
  } catch (error) {
    snapshot.error = typeof error.message === 'string' && /^MAC_DEV_XATTR_[A-Z_]+$/.test(error.message)
      ? error.message : 'MAC_DEV_XATTR_COLLECTION_INVALID';
    snapshot.failure ??= { path: current, kind: 'collection_or_protocol' };
  }
  return snapshot;
}

function validSnapshot(snapshot) {
  if (!snapshot || !Array.isArray(snapshot.nodes) || !snapshot.nodes.length || snapshot.nodes.length > 128) return false;
  const paths = new Map();
  for (const node of snapshot.nodes) {
    if (!node || typeof node.path !== 'string' || node.path.length > 32768
      || node.path !== '.' && (node.path.startsWith('/') || node.path.split('/').some(part => !part || part === '.' || part === '..'))
      || !['file', 'directory'].includes(node.kind) || paths.has(node.path)
      || !Array.isArray(node.attributes) || node.attributes.length > 64) return false;
    const names = new Set();
    for (const attribute of node.attributes) {
      if (!attribute || typeof attribute.nameHex !== 'string' || !/^(?:[0-9a-f]{2})+$/.test(attribute.nameHex)
        || attribute.nameHex.length > 131072 || Buffer.from(attribute.nameHex, 'hex').includes(0)
        || names.has(attribute.nameHex) || !Number.isInteger(attribute.bytes) || attribute.bytes < 0 || attribute.bytes > 65536
        || typeof attribute.sha256 !== 'string' || !/^[0-9a-f]{64}$/.test(attribute.sha256)) return false;
      names.add(attribute.nameHex);
    }
    paths.set(node.path, node);
  }
  if (paths.get('.')?.kind !== 'directory') return false;
  for (const node of paths.values()) {
    if (node.path !== '.' && paths.get(node.path.includes('/') ? node.path.slice(0, node.path.lastIndexOf('/')) : '.')?.kind !== 'directory') return false;
  }
  return true;
}

function matchingSignatureProvenance(previous, current) {
  const names = ['Contents/_CodeSignature', 'Contents/_CodeSignature/CodeResources'];
  const added = [...current.keys()].filter(name => !previous.has(name)).sort(compare);
  if (JSON.stringify(added) !== JSON.stringify(names)
    || current.get(names[0])?.kind !== 'directory' || current.get(names[1])?.kind !== 'file') return false;
  const key = Buffer.from('com.apple.provenance').toString('hex');
  const anchors = ['.', 'Contents'].map(name => previous.get(name));
  if (anchors.some(node => node?.kind !== 'directory')) return false;
  const attributes = anchors.map(node => node.attributes.filter(attribute => attribute.nameHex === key));
  if (attributes.some(items => items.length !== 1) || attributes[0][0].bytes === 0) return false;
  const anchor = attributes[0][0];
  const matches = attribute => attribute.nameHex === key && attribute.bytes === anchor.bytes && attribute.sha256 === anchor.sha256;
  return matches(attributes[1][0]) && names.every(name => current.get(name).attributes.length === 1 && matches(current.get(name).attributes[0]));
}

export function compareAttributeSnapshots(before, after, { allowMatchingSignatureProvenance = false } = {}) {
  if (before?.complete !== true || after?.complete !== true) return { allowed: false, error: after?.error || before?.error || 'MAC_DEV_XATTR_COLLECTION_INCOMPLETE', differences: [] };
  if (!validSnapshot(before) || !validSnapshot(after)) return { allowed: false, error: 'MAC_DEV_XATTR_SNAPSHOT_INVALID', differences: [] };
  const previous = new Map(before.nodes.map(node => [node.path, node]));
  const current = new Map(after.nodes.map(node => [node.path, node]));
  const differences = [], allowedNewSignatureNodes = [];
  const matchingProvenance = allowMatchingSignatureProvenance === true && matchingSignatureProvenance(previous, current);
  for (const [name, node] of previous) {
    const next = current.get(name);
    if (!next) { differences.push({ path: name, kind: 'existing_node_removed' }); continue; }
    if (node.kind !== next.kind) differences.push({ path: name, kind: 'node_type_changed' });
    const oldAttributes = new Map(node.attributes.map(item => [item.nameHex, item]));
    const newAttributes = new Map(next.attributes.map(item => [item.nameHex, item]));
    for (const key of [...new Set([...oldAttributes.keys(), ...newAttributes.keys()])].sort(compare)) {
      const old = oldAttributes.get(key), newer = newAttributes.get(key);
      if (!old || !newer || old.bytes !== newer.bytes || old.sha256 !== newer.sha256) differences.push({ path: name,
        nameHex: key, kind: !old ? 'attribute_added' : !newer ? 'attribute_removed' : 'attribute_value_changed' });
    }
  }
  for (const [name, node] of current) {
    if (previous.has(name)) continue;
    const expected = name === 'Contents/_CodeSignature' && node.kind === 'directory'
      || name === 'Contents/_CodeSignature/CodeResources' && node.kind === 'file';
    if (!expected || (node.attributes.length && !matchingProvenance) || current.get('Contents/_CodeSignature/CodeResources')?.kind !== 'file') {
      differences.push({ path: name, kind: expected ? 'new_signature_node_attributes_or_layout_require_review' : 'unexpected_new_node' });
    } else allowedNewSignatureNodes.push(name);
  }
  const error = !differences.length ? null : differences.some(item => ['attribute_added', 'attribute_removed', 'attribute_value_changed', 'existing_node_removed', 'node_type_changed'].includes(item.kind))
    ? 'MAC_DEV_XATTR_CHANGED' : differences.some(item => item.kind === 'unexpected_new_node')
      ? 'MAC_DEV_XATTR_UNEXPECTED_NODES' : 'MAC_DEV_XATTR_NEW_SIGNATURE_ATTRIBUTES_REVIEW';
  return { allowed: differences.length === 0, error,
    scope: 'attribute_comparison_only',
    existingAttributesUnchanged: !differences.some(item => ['attribute_added', 'attribute_removed', 'attribute_value_changed', 'existing_node_removed', 'node_type_changed'].includes(item.kind)),
    newSignatureProvenanceMatched: matchingProvenance && differences.length === 0,
    ...(matchingProvenance ? { provenanceAnchors: ['.', 'Contents'], provenanceEvidence: 'nonempty_length_and_sha256_equality_only' } : {}),
    differences: differences.sort((a, b) => compare(a.path, b.path) || compare(a.nameHex || '', b.nameHex || '')),
    allowedNewSignatureNodes: allowedNewSignatureNodes.sort(compare) };
}
