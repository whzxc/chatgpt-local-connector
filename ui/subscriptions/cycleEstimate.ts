import type {ProviderSnapshot, QuotaWindow} from './types';
import {quotaPeriod} from './quotaTiming';
import {t, locale} from '../i18n';

export function cycleEstimate(w:QuotaWindow,p:ProviderSnapshot,now:number):{amount:string;lines:string[]}|null {
  const h=p.history, length=quotaPeriod(w,p), reset=Date.parse(w.resetsAt??'');
  const observed=Date.parse(p.observedAt??''), historyAt=Date.parse(h?.observedAt??'');
  const local = p.agentId==='codex' && h?.scope==='local-device';
  if(!h?.samples || h.error || (!local && (h.incomplete || h.scope!=='account-export')) || p.state!=='ready' || p.error || !length
    || !Number.isFinite(reset) || !Number.isFinite(observed) || !Number.isFinite(historyAt)
    || now-observed>300000 || now-historyAt>300000 || observed>now+30000 || historyAt>now+30000
    || now>=reset || w.usedPercent<1 || w.usedPercent>=100 || !Number.isFinite(w.usedPercent))return null;
  // Account-wide history cannot price a model-specific or separate quota pool.
  if((new Set(p.windows.map(window=>window.poolId)).size>1 && !(p.agentId==='codex' && w.poolId==='codex') && !(p.agentId==='cursor' && w.id==='plan')) || w.label==='weekly_scoped')return null;
  const start=reset-length, end=Math.min(observed,historyAt,now);
  if(start>=end || !h.coverageStart || Date.parse(h.coverageStart)>start
    || Math.abs(observed-historyAt)>300000)return null;
  const sample=h.samples[w.id];
  if(!sample || sample.start!==start || sample.end!==end)return null;
  const {tokens,estimatedUsd:usd,pricedTokens:priced}=sample;
  if(tokens<=0 || priced<=0 || (!local && priced!==tokens) || usd<=0 || !Number.isFinite(usd))return null;
  const money=new Intl.NumberFormat(locale.get(),{style:'currency',currency:'USD',currencyDisplay:'narrowSymbol',maximumFractionDigits:2});
  const compact=new Intl.NumberFormat(locale.get(),{notation:'compact',maximumFractionDigits:1});
  const amount=money.format(usd*100/w.usedPercent);
  const date=new Intl.DateTimeFormat(locale.get(),{month:'2-digit',day:'2-digit',hour:'2-digit',minute:'2-digit'});
  return {amount,lines:[t('usageCycleEstimate',{amount}),
    t('usageCycleRange',{start:date.format(start),end:date.format(end)}),
    t('usageCycleSample',{tokens:compact.format(priced),amount:money.format(usd),percent:w.usedPercent}),
    ...(local ? [t('usageCycleLocal')] : []),
    ...(h.incomplete || priced!==tokens ? [t('usageCycleIncomplete')] : [])]};
}
