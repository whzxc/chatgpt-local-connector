import { useEffect, useRef, useState } from 'react';
import { ChartColumn, ArrowLeft, RefreshCw, Layers, Link, Clock, AlertCircle } from 'lucide-react';
import { Button, Icon, SingleChoice, Progress, Notice } from '../components/ui';
import { initialize, onResult, refresh, type Snapshot, type Counts, type Detail } from './bridge';
import '../style.css';
import './style.css';
const number=(n:number|null|undefined)=>n==null?'未知':n.toLocaleString('zh-CN',{maximumFractionDigits:1});
const date=(n:number|string|null|undefined)=>n==null?'未知':new Date(n).toLocaleString('zh-CN',{month:'2-digit',day:'2-digit',hour:'2-digit',minute:'2-digit'});
const short=(id:string)=>id.slice(0,8);
const issues:Record<string,string>={
 'legacy-coverage-response-and-compaction-unknown':'旧日志：response 身份和压缩用量覆盖未知',
 'legacy-related-history-partial':'旧父子／分叉日志：仅列可确认的新增量',
 'legacy-inherited-baseline-unknown':'继承基线缺失，相关用量未计入',
 'legacy-overlap-baseline-conflict':'重叠的旧日志累计基线存在冲突，相关合计未知',
 'legacy-counter-reset-gap':'累计计数发生重置，跨边界增量未知',
 'legacy-prefix-excluded-from-response-family':'存在旧格式前缀；仅汇总新版 response，早期覆盖不完整',
 'foreign-thread-record-excluded':'继承的其他线程记录已排除',
 'partial-tail-pending':'末行仍在写入，等待完整记录',
 'conflicting-response-usage':'相同 response 出现冲突，保留原始字段，相关合计未知',
 'invalid-or-missing-token-fields':'存在缺失字段或包含关系异常',
 'oversized-line-not-indexed':'存在超过读取上限的日志行，覆盖不完整',
 'response-sum-cumulative-mismatch':'已读 response 合计与原生累计不一致，历史覆盖需核对',
 'compaction-response-not-observed':'压缩引用对应的独立 response 尚未读取',
 'source-rewritten-history-coverage-unknown':'源文件被改写／替换，已重建；删除的历史覆盖未知',
 'legacy-missing-cumulative-baseline':'旧记录缺少累计基线，无法确认本次增量',
 'missing-session-identity':'部分日志缺少原生任务身份',
 'missing-response-id':'部分新版记录缺少 response 身份，未加入合计',
 'missing-turn-id':'部分用量记录无法关联到轮次',
 'missing-event-time':'部分记录缺少事件时间，未加入区间统计',
 'invalid-json-line':'存在无法解析的日志行，已跳过',
 'file-unreadable':'部分文件暂不可读，保留上次快照',
 'file-read-failed':'读取中断，等待下一次扫描',
 'directory-unreadable':'部分日志目录暂不可读',
 'file-discovery-limit':'本次发现达到文件数量上限，覆盖不完整',
 'file-metadata-unavailable':'部分文件状态无法获取',
 'file-seek-failed':'部分文件无法恢复读取位置',
 'checkpoint-unavailable':'统计检查点无法保存，重启后将重新读取',
};
function Metrics({value}:{value:Counts}){
 return <div className="metrics"><div><span>输入</span><strong>{number(value.input)}</strong><small>其中缓存 {number(value.cached)}</small></div><div><span>输出</span><strong>{number(value.output)}</strong><small>其中推理 {number(value.reasoning)}</small></div><div><span>{value.total==null && value.knownTotal!=null?'可确认 token 小计':'总 token'}</span><strong>{number(value.total??value.knownTotal)}</strong><small>{value.uncertainRecords?`另有 ${value.uncertainRecords} 条不确定记录`:`输入 + 输出 · ${number(value.records)} 条记录`}</small></div></div>;
}
function Coverage({values}:{values:string[]}){return values.length>0?<details className="coverage"><summary><Icon icon={AlertCircle}/> 数据覆盖 · {values.length} 项说明</summary>{values.map(v=><p key={v}>{issues[v]??'部分记录覆盖未知，请核对数据来源'}</p>)}</details>:null;}
export default function App(){
 const [data,setData]=useState<Snapshot>();const [error,setError]=useState('');const [busy,setBusy]=useState(false);
 const [days,setDays]=useState('7');const [selected,setSelected]=useState('');const [turnId,setTurnId]=useState('');const [page,setPage]=useState(0);const [responsePage,setResponsePage]=useState(0);const [toolPage,setToolPage]=useState(0);
 const entryScope=useRef<'global'|'thread'>('thread');const receivedInitial=useRef(false);
 const current=useRef(data);current.current=data;const alive=useRef(true);const request=useRef(0);const initialized=useRef(false);const inFlight=useRef(false);
 const filter=useRef({days,selected,turnId,page,responsePage,toolPage});filter.current={days,selected,turnId,page,responsePage,toolPage};
 const accept=(next:Snapshot)=>{if(alive.current){setData(next);setError('');}};
 const update=async(mode:'auto'|'filter'|'manual'='manual')=>{
   if(!initialized.current || (inFlight.current && mode!=='filter'))return;
   inFlight.current=true;const serial=++request.current;setBusy(true);
   const f=filter.current;
   try{const next=await refresh({scope:f.selected?'thread':entryScope.current,days:Number(f.days),...(f.selected?{threadId:f.selected}:{}),...(f.turnId?{turnId:f.turnId}:{}),turnOffset:f.page*20,responseOffset:f.responsePage*50,toolOffset:f.toolPage*50});if(serial===request.current)accept(next);}
   catch(e){if(alive.current && serial===request.current)setError(e instanceof Error?e.message:'刷新失败');}
   finally{if(alive.current && serial===request.current){inFlight.current=false;setBusy(false);}}
 };
 const updateRef=useRef(update);updateRef.current=update;
 useEffect(()=>{
  alive.current=true;
  const off=onResult(next=>{if(!receivedInitial.current){entryScope.current=next.scope;receivedInitial.current=true;}accept(next);});
  void initialize().then(()=>{initialized.current=true;}).catch(e=>setError(String(e)));
  const timer=setInterval(()=>{if(document.visibilityState==='visible' && current.current)void updateRef.current('auto');},10000);
  return()=>{alive.current=false;off();clearInterval(timer);};
 },[]);
 useEffect(()=>{if(current.current)void updateRef.current('filter');},[days,selected,turnId,page,responsePage,toolPage]);
 const choose=(id:string)=>{if(id==='unknown'||id===filter.current.selected)return;request.current++;inFlight.current=false;setData(previous=>previous?{...previous,thread:null,binding:'unknown'}:previous);setSelected(id==='unknown'?'':id);setTurnId('');setPage(0);setResponsePage(0);setToolPage(0);};
 const global=data?.scope==='global' && !selected;
 const title=global?'Connector 总览':'任务用量';
 const quotaStale=!!data?.quota && (!data.quota.selected || data.quota.state!=='ready' || !data.quota.observedAt || Date.now()-Date.parse(data.quota.observedAt)>300000);
 const changed=data?.state==='ready' && data.thread && data.binding==='host';
 return <main className="usage-app">
  <header><div className="heading"><Icon icon={ChartColumn} size={20}/><h1>{data?title:'Connector'}</h1></div><Button onClick={()=>void update()} busy={busy} aria-label="刷新用量"><Icon icon={RefreshCw}/>刷新</Button></header>
  <p className="subtitle">本设备原生日志 · 只读观察 · 刷新无需模型轮次</p>
  {error&&<Notice>{error} {data?.state==='ready'?'下方保留上次快照。':''}</Notice>}
  {!data&&<section className="empty"><Icon icon={ChartColumn} size={20}/><h2>正在连接宿主与 Connector…</h2><p>首次索引需要读取本设备日志。插件会自动启动本机后台。</p></section>}
  {data && data.state!=='ready'&&<section className="empty"><Icon icon={Link} size={20}/><h2>{data.state==='incompatible'?'版本需要同步':data.state==='collecting'?'正在索引本机日志':'等待本机后台'}</h2><p>{data.message}</p><Button onClick={()=>void update()}>重新连接</Button></section>}
  {data?.state==='ready'&&<>
   <div className="freshness"><span className={error?'dot stale':'dot'}/><span>最近扫描 {date(data.observedAt)}</span><span>Connector {data.connectorVersion}</span></div>
   {global?<>
    <section><div className="section-heading"><h2>账号额度窗口</h2><small>来自当前原生登录 · 跨设备账号范围</small></div>
    {!data.quota?.windows.length?<p className="muted">额度未获取。可在主应用的 Agent 订阅用量设置中查看来源状态。</p>:data.quota.windows.map(w=><div className="quota" key={w.id}><div className="between"><strong>{w.poolId} · {/^(\d+) min$/.test(w.label)&&Number(w.label.split(' ')[0])>=1440?`${Number(w.label.split(' ')[0])/1440} 天窗口`:w.label}</strong><span>已用 {number(w.usedPercent)}%</span></div><Progress value={w.usedPercent} label={`${w.poolId} 已用 ${w.usedPercent}%`}/><small>重置 {date(w.resetsAt)} · 读取 {date(data.quota?.observedAt)} · {quotaStale?'缓存已过期／暂停':'已读取'}</small></div>)}
    <p className="note">账号百分比与下方本设备日志覆盖不同，不能分摊为任务扣费。</p></section>
    <section><div className="section-heading"><h2>区间用量</h2><SingleChoice label="时间范围" value={days} onChange={setDays} options={[{value:'1',label:'近 24 小时'},{value:'7',label:'近 7 天'},{value:'30',label:'近 30 天'}]}/></div><p className="note">按事件时间统计 · 日分组 UTC · {date(data.range.start)} — {date(data.range.end)}</p><Metrics value={data.usage}/>
     <div className="daily" aria-label="每日 token（UTC）">{data.daily.map(d=><div key={d.day} title={`${d.day} UTC · ${number(d.usage.total)} token`}><div style={{height:`${Math.max(2,70*(d.usage.total??0)/Math.max(1,...data.daily.map(v=>v.usage.total??0)))}px`}}/><small>{d.day.slice(5)}</small></div>)}</div>
    </section>
    <section><div className="section-heading"><h2>查看本机任务</h2><small>选择本设备记录，或查看区间排名</small></div>
    <SingleChoice label="打开本机任务用量" value="unknown" onChange={choose} options={[{value:'unknown',label:'请选择本机任务'},...data.tasks.map(t=>({value:t.id,label:`${t.label} · ${date(t.lastEventAt)}`}))]}/>
    <p className="note">任务右侧面板适用于本机 Codex 任务。当前宿主在云端任务中可能无法加载本地插件；可从这里查看本机任务。所选数据不代表云端任务用量。</p>
    <p className="note">区间 token 前 20 项 · 高用量不等于浪费</p>
    {data.tasks.length===0?<p className="muted">尚未找到本设备可读任务日志。</p>:data.tasks.filter(t=>t.period.records>0).slice(0,20).map(t=><Button className="task-row" variant="ghost" key={t.id} onClick={()=>choose(t.id)}><span><strong>{t.label}</strong><small>{date(t.lastEventAt)} · {t.family==='response'?'response 记录':'旧累计差分'}{t.issues.length?' · 有覆盖说明':''}</small></span><span><strong>{number(t.period.total)}</strong><small>lifetime {number(t.lifetime.total)}</small></span></Button>)}
    </section>
    <section><h2>模型配置维度</h2><p className="note">记录关联的请求配置；不代表已确认的实际执行模型或性价比。</p>{data.models.map(m=><div className="model-row" key={m.name}><span>{m.name}</span><strong>{number(m.usage.total)} token</strong></div>)}</section>
   </>:<>
    {selected&&<Button variant="ghost" onClick={()=>choose('')}><Icon icon={ArrowLeft}/>返回总览／宿主任务</Button>}
    <section className="binding"><div className="between"><h2>{changed?'已绑定当前原生任务':data.binding==='selected'?'已手动选择任务':data.binding==='conflict'?'宿主身份冲突':'当前任务身份未知'}</h2><Icon icon={Link}/></div><p className="note">{changed?`任务 ${data.thread?.id}`:'选择只影响当前面板；不会把最近任务当作当前任务。'}</p>
    <SingleChoice label="选择本设备任务" value={selected || (changed?data.thread!.id:'unknown')} onChange={choose} options={[{value:'unknown',label:'请选择任务'},...data.tasks.map(t=>({value:t.id,label:`${t.label} · ${date(t.lastEventAt)}`}))]}/>
    <p className="note">可选任务 {data.tasks.length} / {data.taskCount}；超出时仅列区间排序前 500 项。</p></section>
    {data.thread?<TaskDetail detail={data.thread} turnId={turnId} selectTurn={id=>{setTurnId(id);setResponsePage(0);setToolPage(0);}} page={page} setPage={setPage} responsePage={responsePage} setResponsePage={setResponsePage} toolPage={toolPage} setToolPage={setToolPage}/>:<section className="empty"><h2>没有可关联的本机任务</h2><p>云端、其他设备、global helper 及尚无原生日志的任务不自动映射。</p></section>}
   </>}
   <Coverage values={data.issues}/><footer>采集范围：本机可读 sessions 与 archived_sessions。没有完整云端／其他设备用量；账号归属覆盖未知。缓存属于输入，推理属于输出，不能再次加总。子任务单独列示。</footer>
  </>}
 </main>;
}
function TaskDetail({detail:d,turnId,selectTurn,page,setPage,responsePage,setResponsePage,toolPage,setToolPage}:{detail:Detail;turnId:string;selectTurn:(s:string)=>void;page:number;setPage:(n:number)=>void;responsePage:number;setResponsePage:(n:number)=>void;toolPage:number;setToolPage:(n:number)=>void}){
 return <>
 <section><div className="section-heading"><h2>本任务 lifetime</h2><small>{d.family==='response'?'按 thread + response 去重':'旧累计差分 · 覆盖不完整'}</small></div><Metrics value={d.usage}/><Coverage values={d.issues}/><p className="note">线程 credits：未知（未取得） · API 等价价格、账号变化均不作为实际扣费。日志写入版本 {d.cliVersion??'未知'}。</p></section>
 <section><div className="section-heading"><h2><Icon icon={Clock}/>轮次</h2><small>{d.turnCount} 轮 · 选择轮次查看关联记录</small></div>
 {d.turns.length===0?<p className="muted">日志没有轮次身份记录。</p>:d.turns.map(t=><Button key={t.id} variant="ghost" className={`turn-row ${turnId===t.id?'active':''}`} onClick={()=>selectTurn(turnId===t.id?'':t.id)}><div className="between"><strong>{short(t.id)} · {status(t.status)}</strong><span>{number(t.usage.total)} token</span></div><small>{date(t.startedAt)} · 请求 {t.model??'未知'} / {t.effort??'未知'} · 工具 {t.toolCount}</small><div className="timing"><span>整轮 {t.durationMs==null?'未知':`${number(t.durationMs/1000)} s`}</span><span>TTFT {t.ttftMs==null?'未知':`${number(t.ttftMs/1000)} s`}</span><span>整轮平均输出 {number(t.wholeTurnOutputTps)} tok/s</span></div></Button>)}
 <Pager page={page} count={d.turnCount} size={20} change={setPage}/><p className="note">整轮时间含工具与等待，平均输出速度不是纯生成速度。运行状态是最后一个日志事件，完成不代表业务验收。</p></section>
 <section><div className="section-heading"><h2>Response 明细</h2>{turnId&&<Button variant="ghost" onClick={()=>selectTurn('')}>显示全部轮次</Button>}</div><p className="note">{turnId?`轮次 ${short(turnId)}`:'全部轮次'} · 新版 response 用量优先，旧 token_count 与压缩引用不重复加入。</p>
 {d.responses.map(r=><details className="record" key={r.id}><summary><span>{short(r.id)} · {date(r.at)}</span><strong>{number(r.tokens.input==null||r.tokens.output==null?null:r.tokens.input+r.tokens.output)} token</strong></summary><dl><dt>输入 / 缓存子集</dt><dd>{number(r.tokens.input)} / {number(r.tokens.cached)}</dd><dt>输出 / 推理子集</dt><dd>{number(r.tokens.output)} / {number(r.tokens.reasoning)}</dd><dt>请求模型 / effort</dt><dd>{r.model??'未知'} / {r.effort??'未知'}</dd><dt>Service tier</dt><dd>{r.serviceTier??'未知'}</dd><dt>轮次 / 来源</dt><dd>{r.turnId??'未知'} / {r.family}</dd><dt>实际执行模型</dt><dd>未知</dd>{!r.reliable&&<><dt>字段状态</dt><dd>冲突／异常，未作为可信合计</dd></>}</dl></details>)}<Pager page={responsePage} count={d.responseCount} size={50} change={setResponsePage}/></section>
 <section><h2>工具关联</h2><p className="note">按 call ID 对齐返回。字节不是 token，返回不等于成功，没有独立的“工具账单”。</p>
 {d.tools.length===0?<p className="muted">此范围没有可关联工具记录。</p>:d.tools.map(t=><div className="tool-row" key={t.id}><div className="between"><strong>{t.name??'工具名未知'}</strong><span>{t.outputBytes==null?'等待返回 / 未记录':`${number(t.outputBytes)} B`}</span></div><small>{short(t.id)} · {t.status??(t.outputBytes==null?'结果未知':'已返回，成功状态未知')} · {t.startedAt!=null&&t.completedAt!=null?`${number((t.completedAt-t.startedAt)/1000)} s`:'耗时未知'}</small>{t.outputBytes!=null&&t.outputBytes>=1024*1024&&<p className="signal">大于 1 MiB 的返回，可检查是否适合分页或缩小读取范围。</p>}</div>)}<Pager page={toolPage} count={d.toolCount} size={50} change={setToolPage}/></section>
 {(d.compactions.length>0 || d.children.length>0 || d.parentId || d.forkedFromId)&&<section><h2><Icon icon={Layers}/>压缩与任务关系</h2>{d.compactions.map(c=><p className="note" key={c.responseId}>{date(c.at)} · 压缩引用 {short(c.responseId)}（不重复计量）</p>)}<p className="note">父任务 {d.parentId?short(d.parentId):'未记录'} · 分叉来源 {d.forkedFromId?short(d.forkedFromId):'未记录'} · 可关联子任务 {d.children.length} 个。各任务单独计量，未证明的父子边界不做合并。</p></section>}
 </>;
}
function status(value:string){return ({completed:'已完成',interrupted:'已中断','running-at-last-event':'最后记录：运行中'} as Record<string,string>)[value]??'状态未知';}
function Pager({page,count,size,change}:{page:number;count:number;size:number;change:(n:number)=>void}){return count>size?<div className="pager"><Button disabled={page===0} onClick={()=>change(page-1)}>上一页</Button><small>{page+1} / {Math.ceil(count/size)}</small><Button disabled={(page+1)*size>=count} onClick={()=>change(page+1)}>下一页</Button></div>:null;}
