import test from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import * as React from 'react';
import { createServer } from 'vite';
import type { PolicyRequest } from './executionPolicy.ts';
import type { ProviderReadinessReport } from './types.ts';

// Run actual component handlers and effects with React's hook dispatcher and
// synthetic IPC. No browser, native executable, account or model is contacted.
// SSR alone cannot observe the effects which used to auto-check accounts.
type Slot = { value?: any; deps?: unknown[]; cleanup?: () => void };
type Frame = { type: unknown; slots: Slot[]; index: number };
type Host = { type: string; props: Record<string, any>; children: (Host | string)[] };
function hookRenderer() {
  const internals = (React as any).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
  const frames = new Map<string, Frame>();
  let frame: Frame, changed = false, effects: (() => void)[] = [], hosts: Host[] = [];
  const same = (a?: unknown[], b?: unknown[]) => a !== undefined && b !== undefined && a.length === b.length && a.every((value, index) => Object.is(value, b[index]));
  const slot = () => frame.slots[frame.index] || (frame.slots[frame.index] = {});
  const dispatcher = {
    useState(initial: unknown) { const item = slot(); frame.index++; if (!('value' in item)) item.value = typeof initial === 'function' ? initial() : initial; return [item.value, (update: any) => { const next = typeof update === 'function' ? update(item.value) : update; if (!Object.is(item.value, next)) { item.value = next; changed = true; } }]; },
    useRef(initial: unknown) { const item = slot(); frame.index++; if (!('value' in item)) item.value = { current: initial }; return item.value; },
    useMemo(create: () => unknown, deps?: unknown[]) { const item = slot(); frame.index++; if (!same(item.deps, deps)) { item.value = create(); item.deps = deps; } return item.value; },
    useCallback(callback: unknown, deps?: unknown[]) { return this.useMemo(() => callback, deps); },
    useEffect(create: () => void | (() => void), deps?: unknown[]) { const item = slot(); frame.index++; if (!same(item.deps, deps)) { item.deps = deps; effects.push(() => { item.cleanup?.(); item.cleanup = create() || undefined; }); } },
    useSyncExternalStore(_subscribe: unknown, snapshot: () => unknown) { return snapshot(); },
  };
  function render(element: any) {
    let pass = 0;
    do {
      changed = false; effects = []; hosts = [];
      const visited = new Set<string>();
      function visit(value: any, path: string): (Host | string)[] {
        if (value == null || typeof value === 'boolean') return [];
        if (Array.isArray(value)) return value.flatMap((item, index) => visit(item, `${path}.${item?.key || index}`));
        if (typeof value !== 'object') return [String(value)];
        if (typeof value.type === 'function') {
          let entry = frames.get(path);
          if (entry && entry.type !== value.type) { entry.slots.forEach(item => item.cleanup?.()); frames.delete(path); entry = undefined; }
          if (!entry) { entry = { type: value.type, slots: [], index: 0 }; frames.set(path, entry); }
          visited.add(path); entry.index = 0; frame = entry;
          const previous = internals.H; internals.H = dispatcher;
          let result: unknown;
          try { result = value.type(value.props); } finally { internals.H = previous; }
          return visit(result, path + '.render');
        }
        const children = visit(value.props?.children, path + '.children');
        if (typeof value.type !== 'string') return children;
        const host = { type: value.type, props: value.props, children }; hosts.push(host); return [host];
      }
      visit(element, 'root');
      for (const [path, entry] of frames) if (!visited.has(path)) { entry.slots.forEach(item => item.cleanup?.()); frames.delete(path); }
      effects.forEach(effect => effect());
      assert.ok(++pass < 20, 'effects must settle');
    } while (changed);
    return hosts;
  }
  return { render, unmount() { frames.forEach(entry => entry.slots.forEach(item => item.cleanup?.())); frames.clear(); } };
}
function content(host: Host | string): string { return typeof host === 'string' ? host : host.children.map(content).join(''); }
function result(provider: ProviderReadinessReport['provider'], model = 'synthetic-model'): ProviderReadinessReport {
  return { schemaVersion: 'provider-readiness/v1', provider, checkedAt: '2026-10-07T00:00:00Z', installation: { state: 'ready', version: 'synthetic-1', executable: '/synthetic/program' }, protocol: { state: 'ready', name: 'synthetic/protocol' }, authentication: { state: provider === 'codex_sdk' ? 'not_checked' : 'authenticated', identityVerified: provider !== 'codex_sdk' }, catalog: { state: provider === 'codex_sdk' ? 'not_checked' : 'available', models: provider === 'codex_sdk' ? [] : [{ id: model, name: model, supportedReasoningEfforts: ['medium', 'high'] }] }, network: { state: 'not_tested', proof: [] }, generation: { enabled: false, testPerformed: false, calls: 0 }, messages: ['Synthetic read-only evidence'], details: provider === 'dreamina' ? { credits: { total: 50 }, freeImageEligibility: 'unverified' } : {} };
}
const settle = () => new Promise(resolve => setImmediate(resolve));

test('actual readiness controls and effects use only the explicit read-only IPC', async t => {
  const calls: PolicyRequest[] = [];
  const readiness = { version: 'provider-readiness/v1', enabled: true, providers: ['codex_cli', 'dreamina', 'codex_sdk'], generationAllowed: false };
  let capability = { mode: 'offline_prepare_only', liveExecutionAllowed: false, stage3CreativePackStatus: 'inactive', reasons: ['synthetic capability'], readiness };
  let respond = async (input: PolicyRequest): Promise<unknown> => result(input.provider as ProviderReadinessReport['provider']);
  Object.assign(globalThis, { isTauri: true, location: { search: '' }, window: { __TAURI_INTERNALS__: { invoke: async (_command: string, args: { request: PolicyRequest }) => { calls.push(args.request); return args.request.action === 'getExecutionPolicy' ? capability : respond(args.request); } } } });
  const server = await createServer({ root: fileURLToPath(new URL('..', import.meta.url)), server: { middlewareMode: true, hmr: false }, appType: 'custom' });
  try {
    const api = await server.ssrLoadModule('/src/api.ts');
    const policy = await server.ssrLoadModule('/src/executionPolicy.ts');
    const { TextExecutionSettings } = await server.ssrLoadModule('/src/TextExecutionSettings.tsx');
    const { SettingsPanel } = await server.ssrLoadModule('/src/Panels.tsx');
    const { validateProviderReadiness } = await server.ssrLoadModule('/src/ProviderReadiness.tsx');
    const { previewWorkspace } = await server.ssrLoadModule('/src/preview.ts');
    const workspace = { ...structuredClone(previewWorkspace), project: { id: 'readiness-project', title: 'Synthetic' }, settings: { codexPath: '/saved/codex', dreaminaPath: '/saved/dreamina' } };
    const props = { workspace, settings: { textProvider: 'codex', codexPath: '/unsaved/CLI path' }, writable: true, update() {}, saveSettings: async () => { throw new Error('readiness must not save settings'); } };
    const button = (nodes: Host[], provider: string) => { const found = nodes.find(node => node.type === 'button' && node.props['data-readiness-provider'] === provider); assert.ok(found, `${provider} button exists`); return found; };
    await t.test('default and startup-probe capabilities stay disabled and run no automatic checks', async () => {
      const view = hookRenderer();
      try {
        policy.installExecutionPolicy(policy.OFFLINE_EXECUTION_POLICY); calls.length = 0;
        for (const provider of ['codex', 'codex_sdk']) {
          const nodes = view.render(React.createElement(TextExecutionSettings, { ...props, settings: { ...props.settings, textProvider: provider } }));
          const check = button(nodes, provider === 'codex' ? 'codex_cli' : 'codex_sdk');
          assert.equal(check.props.disabled, true); check.props.onClick();
        }
        capability = { ...capability, readiness: { ...readiness, enabled: false } };
        await api.request({ action: 'getExecutionPolicy' }); calls.length = 0;
        const nodes = view.render(React.createElement(SettingsPanel, { workspace, writable: true, mutate: async () => { throw new Error('unexpected mutation'); } }));
        assert.equal(button(nodes, 'dreamina').props.disabled, true);
        await settle(); assert.equal(calls.length, 0);
      } finally { view.unmount(); }
    });
    await t.test('generation-off capability enables actual CLI and SDK click handlers using unsaved paths', async () => {
      capability = { ...capability, readiness }; await api.request({ action: 'getExecutionPolicy' }); calls.length = 0;
      const view = hookRenderer();
      try {
        for (const [provider, name, key, path] of [['codex', 'codex_cli', 'codexPath', '/unsaved/CLI path'], ['codex_sdk', 'codex_sdk', 'textSdkNodePath', '/unsaved/Node path']]) {
          const element = React.createElement(TextExecutionSettings, { ...props, settings: { ...props.settings, textProvider: provider, [key]: path } });
          let nodes = view.render(element);
          const check = button(nodes, name); assert.equal(check.props.disabled, false);
          const before = calls.length; await settle(); assert.equal(calls.length, before, 'mount and switch have no IPC');
          check.props.onClick(); check.props.onClick();
          assert.equal(calls.length, before + 1, 'duplicate clicks while pending are ignored');
          assert.deepEqual(calls.at(-1), { action: 'readiness.check', projectId: workspace.project.id, provider: name, settings: { [key]: path } });
          nodes = view.render(element); assert.equal(button(nodes, name).props.disabled, true);
          await settle(); nodes = view.render(element);
          assert.equal(button(nodes, name).props.disabled, false);
          const text = nodes.map(content).join(' ');
          for (const label of ['程序安装', '协议 / 环境', '账户', '模型目录', '网络', '真实模型生成', '未测试 · 本次检查生成调用 0 次']) assert.ok(text.includes(label), label);
          assert.ok(!text.includes('连接成功') && !text.includes('可执行'));
          if (name === 'codex_cli') {
            assert.ok(nodes.some(node => node.type === 'option' && node.props.value === 'synthetic-model'));
            const login = nodes.find(node => node.type === 'button' && node.props['data-execution-action'] === 'textCli.login'); assert.ok(login); assert.equal(login.props.disabled, true); login.props.onClick();
            await settle(); assert.equal(calls.length, before + 1, 'direct blocked login handler cannot save or call IPC');
          } else assert.ok(text.includes('仅证明环境与协议；真实模型未测试'));
        }
      } finally { view.unmount(); }
    });
    await t.test('Jimeng account and provider buttons use readiness with the current unsaved path', async () => {
      calls.length = 0; const view = hookRenderer();
      try {
        const element = React.createElement(SettingsPanel, { workspace, writable: true, mutate: async () => { throw new Error('readiness must not call providers.refresh or dreamina.status'); } });
        let nodes = view.render(element); assert.equal(calls.length, 0);
        const input = nodes.find(node => node.type === 'input' && node.props.value === '/saved/dreamina'); assert.ok(input);
        input.props.onChange({ target: { value: '/unsaved/Jimeng path' } }); nodes = view.render(element);
        const checks = nodes.filter(node => node.type === 'button' && node.props['data-readiness-provider'] === 'dreamina'); assert.equal(checks.length, 2);
        for (const check of checks) assert.equal(check.props.disabled, false);
        checks[1].props.onClick(); await settle(); nodes = view.render(element);
        assert.deepEqual(calls, [{ action: 'readiness.check', projectId: workspace.project.id, provider: 'dreamina', settings: { dreaminaPath: '/unsaved/Jimeng path' } }]);
        assert.ok(nodes.map(content).join(' ').includes('图片免费资格尚未核验'));
        for (const action of ['dreamina.login', 'providers.refresh']) for (const node of nodes.filter(node => node.type === 'button' && node.props['data-execution-action'] === action)) assert.equal(node.props.disabled, true);
      } finally { view.unmount(); }
    });
    await t.test('path, provider and project changes discard pending responses and clear busy', async () => {
      const view = hookRenderer(); let resolve: (value: unknown) => void = () => {};
      respond = () => new Promise(done => { resolve = done; }); calls.length = 0;
      try {
        const initial = React.createElement(TextExecutionSettings, props);
        for (const changed of [
          React.createElement(TextExecutionSettings, { ...props, settings: { ...props.settings, codexPath: '/new/path' } }),
          React.createElement(TextExecutionSettings, { ...props, settings: { textProvider: 'codex_sdk', textSdkNodePath: '/new/node' } }),
          React.createElement(TextExecutionSettings, { ...props, workspace: { ...workspace, project: { id: 'other-project' } } }),
        ]) {
          let nodes = view.render(initial); button(nodes, 'codex_cli').props.onClick();
          nodes = view.render(changed); resolve(result('codex_cli', 'stale-model')); await settle(); nodes = view.render(changed);
          assert.ok(!nodes.map(content).join(' ').includes('stale-model'));
          assert.ok(nodes.filter(node => node.props['data-readiness-provider']).every(node => node.props.disabled === false));
          nodes = view.render(initial); assert.ok(!nodes.map(content).join(' ').includes('stale-model'), 'returning to an earlier path cannot revive results');
        }
        const nodes = view.render(initial); button(nodes, 'codex_cli').props.onClick(); view.unmount(); resolve(result('codex_cli', 'after-unmount')); await settle();
      } finally { view.unmount(); respond = async input => result(input.provider as ProviderReadinessReport['provider']); }
    });
    await t.test('failed and mismatched checks clear busy and allow an explicit retry', async () => {
      const view = hookRenderer(); calls.length = 0;
      const element = React.createElement(TextExecutionSettings, props);
      try {
        respond = async () => { throw new Error('synthetic read-only failure'); };
        button(view.render(element), 'codex_cli').props.onClick(); await settle();
        let nodes = view.render(element);
        assert.equal(button(nodes, 'codex_cli').props.disabled, false);
        assert.ok(nodes.some(node => node.props.role === 'alert' && content(node).includes('synthetic read-only failure')));
        respond = async () => result('dreamina');
        button(nodes, 'codex_cli').props.onClick(); await settle(); nodes = view.render(element);
        assert.equal(button(nodes, 'codex_cli').props.disabled, false);
        assert.ok(nodes.some(node => node.props.role === 'alert' && content(node).includes('只读检查结果无效')));
        respond = async input => result(input.provider as ProviderReadinessReport['provider']);
        button(nodes, 'codex_cli').props.onClick(); await settle(); nodes = view.render(element);
        assert.ok(!nodes.some(node => node.props.role === 'alert'));
        assert.equal(calls.length, 3);
      } finally { view.unmount(); respond = async input => result(input.provider as ProviderReadinessReport['provider']); }
    });
    await t.test('invalid readiness and model-generation claims are rejected; other live actions remain closed', async () => {
      for (const invalid of [null, {}, { ...result('codex_cli'), provider: 'dreamina' }, { ...result('codex_cli'), schemaVersion: 'other' }, { ...result('codex_cli'), generation: { enabled: true, testPerformed: false, calls: 0 } }, { ...result('codex_cli'), generation: { enabled: false, testPerformed: true, calls: 1 } }]) assert.throws(() => validateProviderReadiness(invalid, 'codex_cli'));
      calls.length = 0;
      for (const action of ['textCli.status', 'textCli.login', 'dreamina.status', 'dreamina.login', 'dreamina.logout', 'providers.refresh', 'foundation.run', 'images.run', 'videos.submit']) await assert.rejects(api.request({ action }));
      await assert.rejects(api.request({ action: 'settings.save', settings: { textApiKey: 'synthetic' } }));
      assert.equal(calls.length, 0);
    });
  } finally { await server.close(); }
});
