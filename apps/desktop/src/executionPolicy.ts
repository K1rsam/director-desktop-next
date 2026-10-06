/** The first independent build has no frontend switch that enables live execution.
 * Backend dispatch enforces the same boundary independently of these UI guards.
 */
export type ExecutionPolicy = {
  mode: 'offline_prepare_only';
  liveExecutionAllowed: false;
  stage3CreativePackStatus: 'inactive';
  reasons: string[];
};
export type PolicyRequest = { action: string; [key: string]: unknown };
export type ActionCategory = 'local_read' | 'local_write' | 'generation' | 'provider' | 'credentials' | 'unknown';

export const OFFLINE_EXECUTION_POLICY: ExecutionPolicy = {
  mode: 'offline_prepare_only',
  liveExecutionAllowed: false,
  stage3CreativePackStatus: 'inactive',
  reasons: [
    '当前仅开放本地资料准备与离线验证，产品端真实文本、图片和视频生成保持关闭。',
    '第三阶段创作原件包尚未启用，须完成实际采用来源与适配核验。',
    '渠道、CLI、账户登录及凭据操作保持关闭；视频素材准备尚未形成独立离线路径。',
  ],
};

const LOCAL_READ_ACTIONS = new Set([
  'getExecutionPolicy', 'projects.list', 'files.choose', 'legacy.inspect', 'workspace.get',
  'units.get', 'units.versions.list', 'foundation.choices.list', 'assets.get', 'assets.usage', 'jobs.detail',
]);
const LOCAL_WRITE_ACTIONS = new Set([
  'projects.create', 'projects.archive', 'projects.restore', 'projects.delete',
  'legacy.import', 'project.import', 'project.export', 'units.version.restore', 'units.save',
  'production.direction.save', 'foundation.choice.confirm', 'assets.save', 'assets.delete',
  'assets.restore', 'jobs.close_unknown', 'source.import', 'source.resolve', 'source.reparse',
  'media.import', 'media.adopt', 'media.adopt_batch', 'images.ensure_current', 'videos.adopt',
  'settings.save', 'backup.create', 'backup.restore',
]);
const GENERATION_ACTIONS = new Set([
  'foundation.run', 'foundation.choices.apply', 'images.design', 'images.produce', 'images.run',
  'storyboard.run', 'storyboard.decision.resolve', 'units.regenerate', 'workflows.resume', 'videos.submit',
]);
const PROVIDER_ACTIONS = new Set([
  'providers.refresh', 'videos.preflight', 'videos.prepare', 'jobs.query', 'jobs.download', 'jobs.reconcile',
]);
const CREDENTIAL_ACTIONS = new Set([
  'textCli.status', 'textCli.login', 'textCli.loginStatus', 'textCli.cancelLogin', 'textCli.openLogin',
  'dreamina.status', 'dreamina.login', 'dreamina.logout', 'dreamina.open_authorization',
]);
export const CREDENTIAL_SETTING_KEYS = ['bhToken', 'chromeToken', 'textApiKey'] as const;

export function classifyAction(input: PolicyRequest | string): ActionCategory {
  const request = typeof input === 'string' ? { action: input } : input;
  if (request.action === 'settings.save' && request.settings && typeof request.settings === 'object') {
    const settings = request.settings as Record<string, unknown>;
    if (CREDENTIAL_SETTING_KEYS.some(key => settings[key] != null && settings[key] !== '')) return 'credentials';
  }
  if (LOCAL_READ_ACTIONS.has(request.action)) return 'local_read';
  if (LOCAL_WRITE_ACTIONS.has(request.action)) return 'local_write';
  if (GENERATION_ACTIONS.has(request.action)) return 'generation';
  if (PROVIDER_ACTIONS.has(request.action)) return 'provider';
  if (CREDENTIAL_ACTIONS.has(request.action)) return 'credentials';
  return 'unknown';
}

export function actionBlockReason(input: PolicyRequest | string): string | null {
  switch (classifyAction(input)) {
    case 'local_read': case 'local_write': return null;
    case 'generation': return '离线准备模式：产品端真实文本、图片与视频生成尚未开放。';
    case 'provider': return '离线准备模式：渠道访问、远端查询与渠道素材准备尚未开放。';
    case 'credentials': return '离线准备模式：CLI、账户登录与凭据读写尚未开放。';
    default: return '离线准备模式：此操作尚未纳入已核验的本地操作范围。';
  }
}

export function assertOfflineAction(input: PolicyRequest): void {
  const reason = actionBlockReason(input);
  if (reason) throw new Error(`${reason}（${input.action}）`);
}

export function validateExecutionPolicy(value: unknown): ExecutionPolicy {
  if (!value || typeof value !== 'object') throw new Error('本地核心未返回有效的执行策略。');
  const policy = value as Record<string, unknown>;
  if (policy.mode !== 'offline_prepare_only' || policy.liveExecutionAllowed !== false || policy.stage3CreativePackStatus !== 'inactive' || !Array.isArray(policy.reasons) || !policy.reasons.every(reason => typeof reason === 'string')) {
    throw new Error('本地核心执行策略与当前离线版本不一致；外部执行保持关闭。');
  }
  return { mode: 'offline_prepare_only', liveExecutionAllowed: false, stage3CreativePackStatus: 'inactive', reasons: [...policy.reasons] };
}

/** Keep the guard ahead of IPC, including direct callers that bypass a disabled button. */
export async function dispatchOfflineRequest<T>(input: PolicyRequest, transport: (input: PolicyRequest) => Promise<T>): Promise<T> {
  assertOfflineAction(input);
  const result = await transport(input);
  return input.action === 'getExecutionPolicy' ? validateExecutionPolicy(result) as T : result;
}
