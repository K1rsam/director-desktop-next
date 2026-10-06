import test from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { createServer } from 'vite';

Object.assign(globalThis, { isTauri: false, location: { search: '' } });

// Real shared components, synthetic inputs only. This is not native browser QA.
test('zero-unit scope excludes any existing units, updates with source and retains navigation', async () => {
  const server = await createServer({ root: fileURLToPath(new URL('..', import.meta.url)), server: { middlewareMode: true, hmr: false }, appType: 'custom' });
  try {
    const { Stage3Panel, storyboardEpisodeScope } = await server.ssrLoadModule('/src/Stage3Panel.tsx');
    const { previewWorkspace } = await server.ssrLoadModule('/src/preview.ts');
    const episodes = [{ id: 'e1' }, { id: 'e2' }, { id: 'e3' }];
    const units = [{ episodeId: 'e1', status: 'partial' }];
    assert.deepEqual(storyboardEpisodeScope(episodes, units, 'empty').selection, ['e2', 'e3']);
    assert.deepEqual(storyboardEpisodeScope(episodes, [...units, { episodeId: 'e2' }], 'empty').selection, ['e3']);
    assert.deepEqual(storyboardEpisodeScope(episodes.slice(0, 1), units, 'empty').selection, []);
    assert.deepEqual(storyboardEpisodeScope(episodes, units, 'e1').selection, ['e1']);
    assert.deepEqual(storyboardEpisodeScope(episodes, units, 'all').selection, ['e1', 'e2', 'e3']);
    assert.equal(storyboardEpisodeScope(episodes, units, 'empty').allEpisodesView, true);
    const workspace = structuredClone(previewWorkspace);
    workspace.source.episodes = episodes.map((episode, index) => ({ ...episode, number: index + 1, sourceNumber: String(index + 1), title: `第${index + 1}集` }));
    workspace.units = [{ ...workspace.units[0], episodeId: 'e1' }];
    let mutations = 0;
    const html = renderToStaticMarkup(createElement(Stage3Panel, { workspace, writable: true, mutate: async () => { mutations++; return workspace; } }));
    assert.match(html, /<option value="empty">还没有分镜的集数 · 2 集<\/option>/);
    for (const title of ['第1集', '第2集', '第3集']) assert.ok(html.includes(title));
    assert.equal(mutations, 0);
  } finally { await server.close(); }
});

test('duplicate confirmation binds exact current version hash parser row and diagnostic', async () => {
  const server = await createServer({ root: fileURLToPath(new URL('..', import.meta.url)), server: { middlewareMode: true, hmr: false }, appType: 'custom' });
  try {
    const { SourceStructure, duplicateEpisodeConfirmations, independentEpisodeRequest } = await server.ssrLoadModule('/src/SourceStructure.tsx');
    const source = { version: 7, parserVersion: 'source-parser/v3', sha256: 'a'.repeat(64), episodes: [{ id: 'e1', number: 1, sourceNumber: '一', title: '第一集' }, { id: 'e2', number: 2, sourceNumber: '一', title: '第一集' }, { id: 'e3', number: 3, sourceNumber: '一', title: '第一集' }], scenes: [], rows: [{ id: 'r1', number: 1, text: '第一集\n', range: { start: 0, end: 4 }, kind: 'format', episodeId: 'e1' }, { id: 'r2', number: 2, text: '第一集\n', range: { start: 4, end: 8 }, kind: 'format', episodeId: 'e2' }, { id: 'r3', number: 3, text: '第一集\n', range: { start: 8, end: 12 }, kind: 'format', episodeId: 'e3' }], diagnostics: [{ code: 'DUPLICATE_EPISODE_NUMBER', rowIds: ['r2'], message: '同号集待确认', blocking: true }, { code: 'DUPLICATE_EPISODE_NUMBER', rowIds: ['r3'], message: '同号集待确认', blocking: true }] };
    const choices = duplicateEpisodeConfirmations(source);
    assert.equal(choices.length, 2);
    const request = independentEpisodeRequest(source, choices[0]);
    assert.equal(request.expectedSourceVersion, 7);
    assert.equal(request.decisions[0].confirmation.headingRowId, 'r2');
    assert.deepEqual([request.decisions[0].start, request.decisions[0].end], [4, 8]);
    assert.equal(request.decisions[0].confirmation.sourceSha256, source.sha256);
    for (const changed of [{ ...source, version: 8 }, { ...source, sha256: 'b'.repeat(64) }, { ...source, parserVersion: 'source-parser/v6' }, { ...source, diagnostics: source.diagnostics.slice(1) }, { ...source, rows: source.rows.map(row => row.id === 'r2' ? { ...row, range: { start: 4, end: 9 } } : row) }]) assert.equal(independentEpisodeRequest(changed, choices[0]), null);
    assert.equal(independentEpisodeRequest(source, { ...choices[0], headingRowId: 'r3' }), null);
    assert.equal(independentEpisodeRequest(source, { ...choices[0], end: 12 }), null);
    assert.equal(duplicateEpisodeConfirmations({ ...source, version: undefined }).length, 0);
    let mutations = 0;
    const html = renderToStaticMarkup(createElement(SourceStructure, { source, writable: true, mutate: async () => { mutations++; }, onReadScope: () => {} }));
    assert.equal((html.match(/保留为独立制作段/g) || []).length, 2);
    assert.equal(mutations, 0);
  } finally { await server.close(); }
});
