import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { compareAttributeSnapshots } from './macos_xattr_proof.mjs';

// Pure comparison fixtures: these tests never inspect or modify filesystem attributes.
// Matching synthetic observations prove neither metadata origin nor platform trust.
const optIn = Object.freeze({ allowMatchingSignatureProvenance: true });
const anchors = ['.', 'Contents'];
const signaturePaths = ['Contents/_CodeSignature', 'Contents/_CodeSignature/CodeResources'];
const provenanceName = 'com.apple.provenance';
const clone = value => structuredClone(value);
const digest = value => createHash('sha256').update(value).digest('hex');
const syntheticBytes = (length, seed = 19) => Buffer.from(
  Array.from({ length }, (_, index) => (index * 37 + seed) % 256));
const attribute = (name, value = syntheticBytes(7)) => ({
  nameHex: Buffer.from(name, 'utf8').toString('hex'), name, bytes: value.length, sha256: digest(value),
});
const nodeAt = (snapshot, path) => {
  const node = snapshot.nodes.find(item => item.path === path);
  assert.ok(node, `fixture node ${path} is required`);
  return node;
};
const removeNode = (snapshot, path) => {
  snapshot.nodes = snapshot.nodes.filter(node => node.path !== path);
};
const provenanceAt = (snapshot, path) => nodeAt(snapshot, path).attributes
  .find(item => item.nameHex === Buffer.from(provenanceName).toString('hex'));

function fixture(value = syntheticBytes(17)) {
  const provenance = attribute(provenanceName, value);
  const before = { complete: true, nodes: [
    { path: '.', kind: 'directory', attributes: [clone(provenance), attribute('user.root.fixture')] },
    { path: 'Contents', kind: 'directory', attributes: [clone(provenance)] },
    { path: 'Contents/MacOS', kind: 'directory', attributes: [] },
    { path: 'Contents/MacOS/fixture-app', kind: 'file', attributes: [attribute('user.file.fixture')] },
    { path: 'Contents/Resources', kind: 'directory', attributes: [] },
    { path: 'Contents/Resources/fixture.txt', kind: 'file', attributes: [] },
  ] };
  const after = clone(before);
  after.nodes.push(
    { path: signaturePaths[0], kind: 'directory', attributes: [clone(provenance)] },
    { path: signaturePaths[1], kind: 'file', attributes: [clone(provenance)] },
  );
  return { before, after };
}

function rejectCase(title, mutate) {
  test(title, () => {
    const pair = fixture();
    mutate(pair);
    const decision = compareAttributeSnapshots(pair.before, pair.after, optIn);
    assert.equal(decision.allowed, false, `${title}: ${JSON.stringify(decision)}`);
  });
}

function mutateBoth(pair, path, mutate) {
  for (const snapshot of [pair.before, pair.after]) mutate(nodeAt(snapshot, path));
}

test('attribute-only opt-in accepts the exact two new signature nodes with matching directory observations', () => {
  const { before, after } = fixture();
  const decision = compareAttributeSnapshots(before, after, optIn);
  assert.equal(decision.allowed, true, JSON.stringify(decision));
  assert.equal(decision.error, null);
  assert.deepEqual(decision.differences, []);
  assert.deepEqual(decision.allowedNewSignatureNodes, signaturePaths);
});

test('matching is derived from each input length and digest, with no fixed byte format or stored value', () => {
  const hashes = new Set();
  for (const length of [1, 2, 8, 17, 32, 257]) {
    for (const seed of [0, 73, 211]) {
      const { before, after } = fixture(syntheticBytes(length, seed));
      hashes.add(provenanceAt(before, '.').sha256);
      assert.equal(compareAttributeSnapshots(before, after, optIn).allowed, true,
        `synthetic length ${length}, seed ${seed}`);
    }
  }
  assert.equal(hashes.size, 18);
});

test('provenance exception requires an explicit boolean true opt-in for every comparison', () => {
  const { before, after } = fixture();
  for (const options of [undefined, {}, { allowMatchingSignatureProvenance: false },
    { allowMatchingSignatureProvenance: 1 }, { allowMatchingSignatureProvenance: 'true' }]) {
    assert.equal(compareAttributeSnapshots(before, after, options).allowed, false);
  }
  assert.equal(compareAttributeSnapshots(before, after, optIn).allowed, true);
  assert.equal(compareAttributeSnapshots(before, after).allowed, false);
});

test('node ordering and optional display names do not replace raw attribute-name identity', () => {
  const { before, after } = fixture();
  for (const snapshot of [before, after]) {
    for (const node of snapshot.nodes) {
      for (const item of node.attributes) delete item.name;
      node.attributes.reverse();
    }
  }
  after.nodes.reverse();
  assert.equal(compareAttributeSnapshots(before, after, optIn).allowed, true);
});

test('comparison leaves its snapshots and option unchanged', () => {
  const { before, after } = fixture();
  const original = clone({ before, after });
  compareAttributeSnapshots(before, after, optIn);
  assert.deepEqual({ before, after }, original);
  assert.deepEqual(optIn, { allowMatchingSignatureProvenance: true });
});

test('unchanged existing attributes remain allowed without invoking a new-node exception', () => {
  const { before } = fixture();
  nodeAt(before, 'Contents/Resources/fixture.txt').attributes.push(attribute('com.apple.quarantine'));
  const after = clone(before);
  after.nodes.reverse();
  for (const options of [undefined, optIn]) {
    const decision = compareAttributeSnapshots(before, after, options);
    assert.equal(decision.allowed, true, JSON.stringify(decision));
    assert.deepEqual(decision.allowedNewSignatureNodes, []);
  }
});

test('already existing signature nodes are subject to the ordinary unchanged-node rule', () => {
  const { after: before } = fixture();
  const decision = compareAttributeSnapshots(before, clone(before), optIn);
  assert.equal(decision.allowed, true, JSON.stringify(decision));
  assert.deepEqual(decision.allowedNewSignatureNodes, []);
});

for (const anchor of anchors) {
  rejectCase(`missing pre-existing ${anchor} anchor does not qualify`, pair => {
    for (const snapshot of [pair.before, pair.after]) removeNode(snapshot, anchor);
  });
  rejectCase(`newly introduced ${anchor} anchor does not qualify`, pair => removeNode(pair.before, anchor));
  rejectCase(`missing provenance on ${anchor} does not qualify`, pair => {
    mutateBoth(pair, anchor, node => { node.attributes = []; });
  });
  rejectCase(`file at ${anchor} is not a directory anchor`, pair => {
    mutateBoth(pair, anchor, node => { node.kind = 'file'; });
  });
  rejectCase(`different provenance digest on ${anchor} does not qualify`, pair => {
    mutateBoth(pair, anchor, node => {
      node.attributes = [attribute(provenanceName, syntheticBytes(17, 99))];
    });
  });
  rejectCase(`different provenance length on ${anchor} does not qualify even with the same digest`, pair => {
    for (const snapshot of [pair.before, pair.after]) provenanceAt(snapshot, anchor).bytes += 1;
  });
  rejectCase(`duplicate provenance attributes on ${anchor} do not collapse into one anchor`, pair => {
    mutateBoth(pair, anchor, node => { node.attributes.push(clone(node.attributes[0])); });
  });
  for (const phase of ['before', 'after']) {
    rejectCase(`duplicate ${anchor} path in ${phase} is rejected`, pair => {
      pair[phase].nodes.push(clone(nodeAt(pair[phase], anchor)));
    });
  }
}

rejectCase('an empty provenance value shared by both anchors and new nodes never qualifies', pair => {
  const empty = attribute(provenanceName, Buffer.alloc(0));
  for (const snapshot of [pair.before, pair.after]) {
    for (const node of snapshot.nodes) {
      node.attributes = node.attributes.map(item => item.nameHex === empty.nameHex ? clone(empty) : item);
    }
  }
});

for (const path of signaturePaths) {
  rejectCase(`only the other signature node is new when ${path} was already present`, pair => {
    pair.before.nodes.push(clone(nodeAt(pair.after, path)));
  });
  rejectCase(`missing new node ${path} does not qualify`, pair => removeNode(pair.after, path));
  rejectCase(`mixed empty/provenance pair with no attributes on ${path} does not qualify`, pair => {
    nodeAt(pair.after, path).attributes = [];
  });
  rejectCase(`different new-node provenance hash at ${path} is rejected`, pair => {
    nodeAt(pair.after, path).attributes = [attribute(provenanceName, syntheticBytes(17, 22))];
  });
  rejectCase(`different new-node provenance length at ${path} is rejected`, pair => {
    provenanceAt(pair.after, path).bytes += 1;
  });
  rejectCase(`duplicate new-node provenance at ${path} is rejected`, pair => {
    nodeAt(pair.after, path).attributes.push(clone(provenanceAt(pair.after, path)));
  });
  rejectCase(`duplicate new-node path ${path} is rejected`, pair => {
    pair.after.nodes.push(clone(nodeAt(pair.after, path)));
  });
  for (const kind of ['symlink', 'other', path === signaturePaths[0] ? 'file' : 'directory']) {
    rejectCase(`new signature node ${path} with kind ${kind} is rejected`, pair => {
      nodeAt(pair.after, path).kind = kind;
    });
  }
  for (const alias of [`./${path}`, `/${path}`, `${path}/`, path.replace('Contents/', 'Contents//'),
    path.replace('Contents/', 'Contents/../Contents/'), path.toLowerCase()]) {
    rejectCase(`new-node alias ${alias} does not qualify as ${path}`, pair => {
      nodeAt(pair.after, path).path = alias;
    });
  }
  for (const name of ['com.apple.quarantine', 'com.apple.FinderInfo', 'com.apple.ResourceFork',
    'com.apple.provenance.extra', 'user.fixture']) {
    rejectCase(`additional ${name} at ${path} is rejected`, pair => {
      nodeAt(pair.after, path).attributes.push(attribute(name));
    });
    rejectCase(`${name} cannot replace provenance at ${path}`, pair => {
      nodeAt(pair.after, path).attributes = [attribute(name, syntheticBytes(17))];
    });
  }
  rejectCase(`display-name spoofing does not authorize another raw key at ${path}`, pair => {
    const item = attribute('user.fixture', syntheticBytes(17));
    item.name = provenanceName;
    nodeAt(pair.after, path).attributes = [item];
  });
}

for (const path of ['Contents/_CodeSignature/Extra', 'Contents/Resources/extra.txt', 'Unexpected']) {
  for (const withProvenance of [false, true]) {
    rejectCase(`extra new node ${path} with ${withProvenance ? 'matching provenance' : 'empty attributes'} is rejected`, pair => {
      pair.after.nodes.push({ path, kind: 'file', attributes: withProvenance
        ? [clone(provenanceAt(pair.before, '.'))] : [] });
    });
  }
}

for (const path of ['.', 'Contents', 'Contents/MacOS/fixture-app']) {
  rejectCase(`removing existing node ${path} still blocks a matching signature pair`, pair => {
    removeNode(pair.after, path);
  });
  rejectCase(`changing existing node kind at ${path} still blocks a matching signature pair`, pair => {
    const node = nodeAt(pair.after, path);
    node.kind = node.kind === 'file' ? 'directory' : 'file';
  });
  rejectCase(`removing an existing attribute at ${path} still blocks a matching signature pair`, pair => {
    nodeAt(pair.after, path).attributes.pop();
  });
  rejectCase(`changing an existing attribute digest at ${path} still blocks a matching signature pair`, pair => {
    nodeAt(pair.after, path).attributes[0].sha256 = digest(syntheticBytes(23, 190));
  });
  rejectCase(`changing an existing attribute length at ${path} still blocks a matching signature pair`, pair => {
    nodeAt(pair.after, path).attributes[0].bytes += 1;
  });
  for (const name of ['com.apple.quarantine', 'com.apple.FinderInfo', 'user.extra']) {
    rejectCase(`adding ${name} to existing ${path} still blocks a matching signature pair`, pair => {
      nodeAt(pair.after, path).attributes.push(attribute(name));
    });
  }
}

rejectCase('matching provenance cannot fall back to an arbitrary existing executable', pair => {
  const provenance = clone(provenanceAt(pair.before, '.'));
  for (const snapshot of [pair.before, pair.after]) {
    for (const path of anchors) nodeAt(snapshot, path).attributes = [];
    nodeAt(snapshot, 'Contents/MacOS/fixture-app').attributes.push(clone(provenance));
  }
});

rejectCase('matching provenance cannot fall back to another existing directory', pair => {
  const provenance = clone(provenanceAt(pair.before, '.'));
  for (const snapshot of [pair.before, pair.after]) {
    nodeAt(snapshot, 'Contents').attributes = [];
    nodeAt(snapshot, 'Contents/Resources').attributes.push(clone(provenance));
  }
});

rejectCase('different anchors cannot each authorize one different signature-node value', pair => {
  const other = attribute(provenanceName, syntheticBytes(17, 133));
  mutateBoth(pair, 'Contents', node => { node.attributes = [clone(other)]; });
  nodeAt(pair.after, signaturePaths[1]).attributes = [clone(other)];
});

for (const change of ['removed', 'value_changed', 'name_changed']) {
  rejectCase(`existing quarantine ${change} is never covered by the provenance exception`, pair => {
    const path = 'Contents/Resources/fixture.txt';
    const quarantine = attribute('com.apple.quarantine', syntheticBytes(13, 41));
    nodeAt(pair.before, path).attributes = [clone(quarantine)];
    nodeAt(pair.after, path).attributes = change === 'removed' ? [] : [change === 'value_changed'
      ? attribute('com.apple.quarantine', syntheticBytes(13, 42))
      : attribute('com.apple.provenance', syntheticBytes(13, 41))];
  });
}

rejectCase('adding provenance to old signature nodes is still an existing-attribute change', pair => {
  for (const path of signaturePaths) {
    pair.before.nodes.push({ ...clone(nodeAt(pair.after, path)), attributes: [] });
  }
});

for (const phase of ['before', 'after']) {
  test(`${phase} completion requires literal boolean true, never a truthy value`, () => {
    for (const value of ['false', 'true', 1, {}, []]) {
      const pair = fixture();
      pair[phase].complete = value;
      assert.equal(compareAttributeSnapshots(pair.before, pair.after, optIn).allowed, false);
    }
  });
  rejectCase(`incomplete ${phase} collection cannot grant a matching-provenance exception`, pair => {
    pair[phase].complete = false;
    pair[phase].error = 'MAC_DEV_XATTR_COLLECTION_INCOMPLETE';
  });
}
