import type { Settings } from './types';

type FantasyMode = 'recommend' | 'off' | 'all' | 'review';
type Props = { settings: Settings; writable: boolean; onChange: (patch: Partial<Settings>) => void };

export function displayedFantasyMode(settings: Settings): FantasyMode {
  const mode = settings.fantasyMode;
  if (mode === 'off' || mode === 'all') return mode;
  return mode === 'review' && settings.requireFantasyReview === true ? 'review' : 'recommend';
}
export function fantasySettingsPatch(mode: FantasyMode): Partial<Settings> {
  return { fantasyMode: mode, requireFantasyReview: mode === 'review' };
}
export function ProductionSettings({ settings, writable, onChange }: Props) {
  return <><label>幻想表达<select disabled={!writable} value={displayedFantasyMode(settings)} onChange={event => onChange(fantasySettingsPatch(event.target.value as FantasyMode))}>
    <option value="recommend">自动采用推荐</option><option value="off">不增加额外幻想</option><option value="all">采用全部候选</option><option value="review">先审阅候选</option>
  </select><small className="control-hint">默认自动采用推荐候选并继续制作。选择先审阅后，会等你确认候选。</small></label>
  <label>项目画风<input disabled={!writable} value={String(settings.projectVisualStyle || settings.visualStyle || '')} onChange={event => onChange({ projectVisualStyle: event.target.value, visualStyle: event.target.value })} placeholder="例如：3D半写实国漫"/><small className="control-hint">新设计读取此画风；最终图片请求只组合一次。修改不重写人工稿、旧图片或历史请求。</small></label>
  <label>制作美术方向<textarea disabled={!writable} value={String(settings.productionDirection || '')} onChange={event => onChange({ productionDirection: event.target.value, productionDirectionConfirmed: false })} placeholder="补充服装、材质与审美方向，不覆盖原文年龄、肤色、体型或时代"/></label>
  <label><input type="checkbox" disabled={!writable || !String(settings.productionDirection || '').trim()} checked={settings.productionDirectionConfirmed === true} onChange={event => onChange({ productionDirectionConfirmed: event.target.checked })}/>确认采用此制作美术方向</label>
  <label>人物图片展示<textarea disabled={!writable} value={typeof settings.imagePresentation === 'string' ? settings.imagePresentation : settings.imagePresentation?.composition || ''} onChange={event => onChange({ imagePresentation: event.target.value })} placeholder="默认：左侧大幅肖像，右侧正／侧／背全身，完整头脚，干净中性背景，无资料栏"/><small className="control-hint">仅用于人物基础与派生形象；资产已有明确构图优先，场景、道具、群体和非人角色不套人物四视图。</small></label></>;
}
