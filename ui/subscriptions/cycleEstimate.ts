import type {ProviderSnapshot, QuotaWindow} from './types';
import {quotaPeriod} from './pace';
import {t, locale} from '../i18n';

export function cycleEstimate(w:QuotaWindow,p:ProviderSnapshot,now:number):string[] {
  const h=p.history, length=quotaPeriod(w,p), reset=Date.parse(w.resetsAt??'');
  const observed=Date.parse(p.observedAt??''), historyAt=Date.parse(h?.observedAt??'');
  if(!h?.timeline || h.error || p.state!=='ready' || p.error || !length
    || !Number.isFinite(reset) || !Number.isFinite(observed) || !Number.isFinite(historyAt)
    || now>=reset || w.usedPercent<1 || w.usedPercent>=100 || !Number.isFinite(w.usedPercent))return [];
  // Account-wide history cannot price a model-specific or separate quota pool.
  if((new Set(p.windows.map(window=>window.poolId)).size>1 && !(p.agentId==='codex' && w.poolId==='codex') && !(p.agentId==='cursor' && w.id==='plan')) || w.label==='weekly_scoped')return [];
  const start=reset-length, end=Math.min(observed,historyAt,now);
  if(start>=end || !h.coverageStart || Date.parse(h.coverageStart)>start
    || Math.abs(observed-historyAt)>300000)return [];
  let tokens=0,usd=0,priced=0;
  for(const [at,count,cost,known] of h.timeline) {
    if(at>=start && at<=end){tokens+=count;usd+=cost;priced+=known;}
  }
  if(tokens<=0 || priced!==tokens || usd<=0 || !Number.isFinite(usd))return [];
  const money=new Intl.NumberFormat(locale.value,{style:'currency',currency:'USD',maximumFractionDigits:2});
  const compact=new Intl.NumberFormat(locale.value,{notation:'compact',maximumFractionDigits:1});
  return [t('usageCycleEstimate',{amount:money.format(usd*100/w.usedPercent)}),
    t('usageCycleSample',{tokens:compact.format(tokens),amount:money.format(usd),percent:w.usedPercent})];
}
