// Burn-rate rules adapted from OpenUsage Support/Pace.swift and WidgetData.meterState (MIT).
import type {ProviderSnapshot, QuotaWindow} from './types';
import {quotaDisplay, resetDisplay} from './displayPreferences';
import {duration, preciseTime, usageColors} from './presentation';
import {t} from '../i18n';
function period(w:QuotaWindow,p:ProviderSnapshot):number|undefined {
  const numeric=/^(\d+) (min|s)$/.exec(w.label);
  if(numeric)return Number(numeric[1])*(numeric[2]==='min'?60000:1000);
  if(['rolling','five_hour','session'].includes(w.label))return 5*3600000;
  if(['weekly','seven_day','weekly_all','weekly_scoped'].includes(w.label))return 7*86400000;
  if(p.agentId==='cursor') {
    const raw=p.rawUsage as {summary?:Record<string,unknown>;usage?:Record<string,unknown>}|undefined;
    const summary=raw?.usage ?? raw?.summary;
    const start=summary?.billingCycleStart ?? summary?.billingCycleStartDate;
    const end=summary?.billingCycleEnd ?? summary?.billingCycleEndDate;
    const date=(v:unknown)=>typeof v==='number' ? (v<1e12?v*1000:v) : typeof v==='string'?Date.parse(v):NaN;
    const length=date(end)-date(start);
    if(Number.isFinite(length)&&length>0)return length;
    return 30*86400000;
  }
  if(w.label==='monthly')return 30*86400000;
}
export function quotaPace(w:QuotaWindow,p:ProviderSnapshot,now:number) {
  if(p.state!=='ready'||p.error||!Number.isFinite(w.usedPercent))return;
  const used=w.usedPercent;
  if(used>=100||w.exhausted||p.accountBlocked||p.blockedPoolIds.includes(w.poolId))return {color:usageColors.exhausted,label:t('usagePaceReached'),tooltip:t('usagePaceReached'),flame:true};
  const length=period(w,p),reset=Date.parse(w.resetsAt??'');
  if(!length||!Number.isFinite(reset)||now>=reset||used<=0)return;
  const elapsed=now-(reset-length);
  if(elapsed<Math.max(60000,length*.01))return;
  const projected=used/elapsed*length;
  if(projected>90&&used<5)return;
  const spare=Math.round(100-projected), danger=projected>100||(projected>90&&spare<1);
  const eta=(100-used)*elapsed/used;
  const label=danger ? eta>0&&eta<reset-now ? t(resetDisplay.value==='time'?'usagePaceLimitAt':'usagePaceLimit',{time:resetDisplay.value==='time'?preciseTime(new Date(now+eta).toISOString()):duration(eta/60000,length<=5*3600000)}) : t('usagePaceAtLimit') : projected>90 ? t('usagePaceSpare',{percent:spare}) : '';
  const tooltip=projected>100?t('usagePaceOver',{percent:Math.round(projected-100)}):projected>90?t('usagePaceUsed',{percent:Math.round(projected)}):t('usagePaceLeft',{percent:spare});
  return {color:danger?usageColors.warning:projected>90?usageColors.caution:usageColors.good,label,tooltip,flame:danger,tick:(quotaDisplay.value==='remaining'?1-elapsed/length:elapsed/length)*100};
}
