import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { request } from './api';
import { actionBlockReason, currentExecutionPolicy, subscribeExecutionPolicy } from './executionPolicy';
import type { ReadinessProvider } from './executionPolicy';
import { Button, formatDate, messageOf } from './components';
import type { ProviderReadinessReport } from './types';

const pathKeys = { codex_cli: 'codexPath', dreamina: 'dreaminaPath', codex_sdk: 'textSdkNodePath' } as const;

/** Reject mismatched or contradictory results instead of displaying an execution claim. */
export function validateProviderReadiness(value: unknown, provider: ReadinessProvider): ProviderReadinessReport {
  const report = value as ProviderReadinessReport | null;
  const nullableText = (text: unknown) => text === null || typeof text === 'string';
  if (!report || report.schemaVersion !== 'provider-readiness/v1' || report.provider !== provider
      || typeof report.checkedAt !== 'string'
      || !['ready', 'missing', 'invalid', 'not_checked'].includes(report.installation?.state)
      || !nullableText(report.installation.version) || !nullableText(report.installation.executable)
      || !['ready', 'failed', 'not_checked'].includes(report.protocol?.state) || !nullableText(report.protocol.name)
      || !['authenticated', 'not_logged_in', 'unknown', 'not_checked'].includes(report.authentication?.state)
      || typeof report.authentication.identityVerified !== 'boolean'
      || !['available', 'unavailable', 'not_checked'].includes(report.catalog?.state)
      || !Array.isArray(report.catalog.models) || !report.catalog.models.every(model => model && typeof model.id === 'string'
        && (model.name === undefined || typeof model.name === 'string')
        && (model.kind === undefined || typeof model.kind === 'string')
        && (model.supportedReasoningEfforts === undefined || Array.isArray(model.supportedReasoningEfforts) && model.supportedReasoningEfforts.every(effort => typeof effort === 'string')))
      || !['verified_read_only', 'not_tested', 'failed'].includes(report.network?.state) || !Array.isArray(report.network.proof)
      || report.generation?.enabled !== false || report.generation?.testPerformed !== false || report.generation?.calls !== 0
      || !Array.isArray(report.messages) || !report.messages.every(message => typeof message === 'string')
      || !report.details || typeof report.details !== 'object') throw new Error('本地核心返回的只读检查结果无效，请重新检查。');
  return report;
}

type CheckState = { scope: string; attempt: number; busy: boolean; result: ProviderReadinessReport | null; error: string };
export function useProviderReadiness(projectId: string, provider: ReadinessProvider, path: string, enabled = true) {
  useSyncExternalStore(subscribeExecutionPolicy, currentExecutionPolicy, currentExecutionPolicy);
  const scope = JSON.stringify([projectId, provider, path, enabled]);
  const currentScope = useRef(scope), sequence = useRef(0), inFlight = useRef<string | null>(null), mounted = useRef(true);
  if (currentScope.current !== scope) { currentScope.current = scope; sequence.current++; inFlight.current = null; }
  const [state, setState] = useState<CheckState>({ scope, attempt: sequence.current, busy: false, result: null, error: '' });
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; sequence.current++; inFlight.current = null; }; }, []);
  const blocked = actionBlockReason({ action: 'readiness.check', provider });
  const check = useCallback(async () => {
    if (!enabled || !mounted.current || currentScope.current !== scope || inFlight.current === scope
        || actionBlockReason({ action: 'readiness.check', provider })) return;
    const attempt = ++sequence.current;
    inFlight.current = scope;
    setState({ scope, attempt, busy: true, result: null, error: '' });
    try {
      const response = await request<unknown>({ action: 'readiness.check', projectId, provider, settings: { [pathKeys[provider]]: path } });
      if (!mounted.current || attempt !== sequence.current || currentScope.current !== scope) return;
      setState({ scope, attempt, busy: false, result: validateProviderReadiness(response, provider), error: '' });
    } catch (reason) {
      if (mounted.current && attempt === sequence.current && currentScope.current === scope) setState({ scope, attempt, busy: false, result: null, error: messageOf(reason) });
    } finally {
      if (attempt === sequence.current) inFlight.current = null;
    }
  }, [enabled, scope, projectId, provider, path]);
  const current = state.scope === scope && state.attempt === sequence.current ? state : { busy: false, result: null, error: '' };
  return { ...current, check, blocked, provider };
}
export type ReadinessCheck = ReturnType<typeof useProviderReadiness>;

export function ReadinessButton({ check, writable, children }: { check: ReadinessCheck; writable: boolean; children: string }) {
  return <Button executionAction="readiness.check" data-readiness-provider={check.provider} icon="refresh" disabled={!writable || check.busy || !!check.blocked} title={check.blocked || undefined} onClick={() => void check.check()}>{check.busy ? '正在只读检查…' : children}</Button>;
}

export function ProviderReadinessResult({ check }: { check: ReadinessCheck }) {
  const result = check.result;
  if (check.error) return <p className="warning" role="alert">{check.error}</p>;
  if (!result) return <p className="control-hint">{check.busy ? '正在读取程序与连接状态，不发起生成。' : check.blocked || '点击检查后读取当前程序路径；打开页面和切换渠道不会访问账户。'}</p>;
  const installed = { ready: '已找到', missing: '未找到', invalid: '路径或程序无效', not_checked: '未检查' };
  const protocol = { ready: '握手通过', failed: '握手失败', not_checked: '未检查' };
  const authentication = { authenticated: '已登录', not_logged_in: '未登录', unknown: '状态未知', not_checked: '未检查' };
  const catalog = { available: '已读取', unavailable: '读取未完成', not_checked: '未检查' };
  const network = { verified_read_only: '只读接口已验证', not_tested: '未测试', failed: '只读接口检查失败' };
  return <div className="provider-readiness-result" role="status" data-readiness-result={result.provider}>
    <p className="control-hint">最近只读检查：{formatDate(result.checkedAt)}</p>
    <dl><div><dt>程序安装</dt><dd>{installed[result.installation.state]}{result.installation.version ? ` · ${result.installation.version}` : ''}</dd></div>
      <div><dt>协议 / 环境</dt><dd>{protocol[result.protocol.state]}{result.protocol.name ? ` · ${result.protocol.name}` : ''}</dd></div>
      <div><dt>账户</dt><dd>{authentication[result.authentication.state]} · {result.authentication.identityVerified ? '身份已核验' : '身份未核验'}</dd></div>
      <div><dt>模型目录</dt><dd>{catalog[result.catalog.state]}{result.catalog.state === 'available' ? ` · ${result.catalog.models.length} 个模型` : ''}</dd></div>
      <div><dt>网络</dt><dd>{network[result.network.state]}</dd></div>
      <div><dt>真实模型生成</dt><dd>未测试 · 本次检查生成调用 0 次</dd></div></dl>
    {result.installation.executable && <p className="control-hint path-value">{result.installation.executable}</p>}
    {result.messages.map((message, index) => <p className="control-hint" key={index}>{message}</p>)}
    {result.provider === 'codex_sdk' && <p className="control-hint">SDK 包导入和 worker 握手仅证明环境与协议；真实模型未测试，SDK 真实生成保持关闭。</p>}
    {result.provider === 'dreamina' && <p className="control-hint">图片免费资格尚未核验；积分余额和模型目录不能证明本次生成免费。</p>}
    {result.provider === 'codex_cli' && <p className="control-hint">账户与模型目录读取不代表真实模型生成已通过。</p>}
    {result.catalog.models.length > 0 && <details className="details"><summary>查看本次模型目录</summary><ul className="model-list">{result.catalog.models.map(model => <li key={model.id}>{model.name || model.id}{model.kind ? ` · ${model.kind}` : ''}</li>)}</ul></details>}
  </div>;
}
