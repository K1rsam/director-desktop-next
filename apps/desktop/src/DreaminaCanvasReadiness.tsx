import { useEffect, useRef, useState } from 'react';
import { choose } from './api';
import { Button, messageOf } from './components';
import { ProviderReadinessResult, ReadinessButton, useProviderReadiness } from './ProviderReadiness';

type Props = { projectId: string; path: string; writable: boolean; update: (key: string, value: string) => void };

/** Canvas is a separate, read-only diagnostic adapter, never a generation provider. */
export function DreaminaCanvasReadiness({ projectId, path, writable, update }: Props) {
  const check = useProviderReadiness(projectId, 'dreamina_canvas', path, writable);
  const scope = JSON.stringify([projectId, path, writable]);
  const currentScope = useRef(scope), sequence = useRef(0), mounted = useRef(true), picking = useRef(false);
  if (currentScope.current !== scope) { currentScope.current = scope; sequence.current++; }
  const [selecting, setSelecting] = useState(false), [pickerError, setPickerError] = useState({ scope, message: '' });
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; sequence.current++; }; }, []);

  async function selectExecutable() {
    if (!writable || !mounted.current || picking.current || currentScope.current !== scope) return;
    const attempt = ++sequence.current;
    picking.current = true; setSelecting(true); setPickerError({ scope, message: '' });
    try {
      const paths = await choose('file');
      if (mounted.current && attempt === sequence.current && currentScope.current === scope && paths[0]) update('dreaminaCanvasPath', paths[0]);
    } catch (reason) {
      if (mounted.current && attempt === sequence.current && currentScope.current === scope) setPickerError({ scope, message: messageOf(reason) });
    } finally {
      picking.current = false;
      if (mounted.current) setSelecting(false);
    }
  }

  return <section className="surface dreamina-canvas-readiness" data-readiness-card="dreamina_canvas">
    <h3>即梦 Canvas CLI · 只读检查</h3>
    <p className="muted">检查版本、schema 协议、auth status 登录状态、account 账户身份和 image models 图片模型目录。</p>
    <p className="control-hint">检查范围：default 配置 / cn 区域</p>
    <label>Canvas CLI 程序<div className="text-cli-path"><input data-canvas-path value={path} disabled={!writable} onChange={event => update('dreaminaCanvasPath', event.target.value)} placeholder="选择 dreamina-canvas，或填写可执行文件的完整路径"/><Button data-canvas-picker icon="source" disabled={!writable || selecting} onClick={() => void selectExecutable()}>{selecting ? '正在选择…' : '选择 Canvas 程序'}</Button></div></label>
    {pickerError.scope === scope && pickerError.message && <p className="warning" role="alert">{pickerError.message}</p>}
    <div className="actions"><ReadinessButton check={check} writable={writable && !selecting}>检查 Canvas CLI、账户与图片模型</ReadinessButton></div>
    <ProviderReadinessResult check={check}/>
    <p className="control-hint">使用当前输入路径，无需先保存。选择程序、修改路径、打开页面和切换渠道不会自动检查；修改后请再次点击检查。</p>
    <p className="control-hint">积分 / 额度未知：当前适配的只读协议没有额度接口；免费资格未核验。Canvas 尚未接入生成桥；目录仅供查看，不改变现有即梦路径、登录记录、历史任务或生成选项。</p>
  </section>;
}
