<script setup lang="ts">
import {provideAgentActivity} from '../composables/useAgentActivity';
import m from '../../shared/usage-panel.json';
import {useWindowSize,useResizeObserver} from '@vueuse/core';
import {computed,inject,onMounted,onUnmounted,ref,watch,nextTick} from 'vue';
import {api} from '../api';
import {t} from '../i18n';
import {isDesktop} from '../platform';
import QuotaRing from '../subscriptions/QuotaRing.vue';
import QuotaBubble from '../subscriptions/QuotaBubble.vue';
import {surfaceStatus} from '../subscriptions/presentation';
import type {ProviderSnapshot} from '../subscriptions/types';
import {useSubscriptions,subscriptionSnapshotKey} from '../subscriptions/useSubscriptions';
import {berth,notchOutline,bubbleShape,path,clamp,type Point} from './geometry';
import type {PanelState,PanelGeometry} from './layout';
import {useSpring} from './spring';
const props=defineProps<{state?:PanelState}>();
const emit=defineEmits<{geometry:[value:PanelGeometry];open:[providerId:string]}>();
const snapshot=inject(subscriptionSnapshotKey,undefined)??useSubscriptions().snapshot;
const activeAgents=provideAgentActivity();
const nativePointer=ref<PanelState>();
const pointer=computed(()=>props.state??nativePointer.value);
const selected=computed(()=>snapshot.value?.settings.enabled?snapshot.value.providers.filter(p=>p.eligible&&p.selected):[]);
const working=computed(()=>selected.value.some(p=>activeAgents.value.includes(p.agentId)));
const heldRows=ref<ProviderSnapshot[]>();
watch(()=>pointer.value?.pressed,v=>{heldRows.value=v?[...selected.value]:undefined;});
const allRows=computed(()=>heldRows.value??selected.value);
const {width,height}=useWindowSize();
const l=computed(()=>pointer.value?.layout),prefs=computed(()=>pointer.value?.preferences);
const page=ref(0),capacity=computed(()=>l.value?.metrics.visibleCount??allRows.value.length),pages=computed(()=>Math.max(1,Math.ceil(allRows.value.length/Math.max(1,capacity.value))));
const rows=computed(()=>allRows.value.slice(page.value*capacity.value,(page.value+1)*capacity.value));
watch(pages,v=>page.value=Math.min(page.value,v-1));
const retainedProvider=ref<string|null>(null),active=computed(()=>rows.value.find(p=>p.providerId===retainedProvider.value));
function nextPage(){if(!pointer.value?.pressed){page.value=(page.value+1)%pages.value;retainedProvider.value=null;}}
const pageRect=computed(()=>{const r=l.value?.rail;if(!r)return {x:0,y:0,width:0,height:0};const s=l.value!.metrics.scale;return l.value!.metrics.horizontal?{x:r.x+r.width-l.value!.metrics.padding-24*s,y:r.y+r.height/2-10*s,width:24*s,height:20*s}:{x:r.x+8*s,y:r.y+r.height-l.value!.metrics.padding-22*s,width:r.width-16*s,height:20*s};});
const measured=ref(117),content=ref<HTMLElement>();
// openness, length, thickness, padding, pitch, roundness, floatingness.
const rail=useSpring([0,238,64,46,88,0,0],m.railResponse,m.railDamping);
const axis=useSpring([0],m.railResponse,m.railDamping);
const turnOffset=useSpring([0,0],m.railResponse,m.railDamping);
watch(l,(next,previous)=>{
 if(!next)return;
 if(!previous||!pointer.value?.pressed){axis.jump([next.metrics.horizontal?1:0]);turnOffset.jump([0,0]);return;}
 if(next.metrics.horizontal!==previous.metrics.horizontal||!!next.notch!==!!previous.notch){
  // Compensate the native frame's immediate landing, then animate only the
  // snap/turn correction. Further mouse motion moves the rail synchronously.
  turnOffset.jump([turnOffset.value.value[0]!+(previous.screenX??0)+previous.rail.x+previous.rail.width/2-(next.screenX??0)-next.rail.x-next.rail.width/2,
   turnOffset.value.value[1]!+(previous.screenY??0)+previous.rail.y+previous.rail.height/2-(next.screenY??0)-next.rail.y-next.rail.height/2]);turnOffset.to([0,0]);
 }
 axis.to([next.metrics.horizontal?1:0]);
});
const card=useSpring([0,0,0,117,0],m.cardResponse,m.cardDamping);
const contentFade=useSpring([1],.18,.9);
const scale=computed(()=>l.value?.metrics.scale||1);
const openness=computed(()=>clamp(rail.value.value[0]!,0,1));
const reveal=computed(()=>clamp(card.value.value[0]!,0,1));
const status=computed(()=>surfaceStatus(rows.value,prefs.value?.warningAt??75));
const alert=computed(()=>prefs.value?.alertColor&&status.value.alert?status.value.color:'#000');
const tint=useSpring([0,0,0],m.railResponse,m.railDamping);
watch([alert,()=>pointer.value?.expanded,()=>pointer.value?.dock],()=>{const color=pointer.value?.expanded||pointer.value?.dock==='floating'?'#000000':alert.value==='#000'?'#000000':alert.value;tint.to([1,3,5].map(i=>parseInt(color.slice(i,i+2),16)));});
const railFill=computed(()=>`rgb(${tint.value.value.map(v=>Math.round(clamp(v,0,255))).join(',')})`);
const railPoints=computed<Point[]>(()=>{
 const layout=l.value;if(!layout)return [];const [o,length,thickness,,,round,floating]=rail.value.value as [number,number,number,number,number,number,number];
 const r=layout.rail,s=scale.value;
 if(layout.notch)return notchOutline(openness.value,r.width,layout.notch.width,layout.notch.height,thickness,round,s).map(([x,y])=>[r.x+x+turnOffset.value.value[0]!,r.y+y+turnOffset.value.value[1]!]);
 const h=m.collapsedLength*s+(length-m.collapsedLength*s)*openness.value,w=m.sliverWidth*s+(thickness-m.sliverWidth*s)*openness.value;
 const angle=clamp(axis.value.value[0]!,0,1)*Math.PI/2,c=Math.cos(angle),sn=Math.sin(angle),mirror=layout.edge==='left'||layout.edge==='bottom'?-1:1;
 return berth(clamp(o,0,1),length,thickness,clamp(round,0,1),clamp(floating,0,1),s).map(([x,y])=>{
  const dx=(x-w/2+(thickness-w)/2)*mirror,dy=y-h/2;
  return [r.x+r.width/2+c*dx+sn*dy+turnOffset.value.value[0]!,r.y+r.height/2-sn*dx+c*dy+turnOffset.value.value[1]!];
 });
});
const ringPositions=computed<Point[]>(()=>{
 const layout=l.value;if(!layout)return [];const r=layout.rail,s=scale.value,v=rail.value.value;
 const progress=clamp(axis.value.value[0]!,0,1),angle=progress*Math.PI/2,c=Math.cos(angle),sn=Math.sin(angle);
 return rows.value.map((_,i)=>{
  const dy=v[3]!+18*s+i*v[4]!+(layout.metrics.item-36*s)/2*progress-v[1]!/2;
  const nh=layout.notch?.height||0;
  const dx=layout.metrics.percentages?11*s*progress:0;
  // The centres follow the rotating layout; glyphs themselves stay upright.
  return [r.x+r.width/2+c*dx+sn*dy+turnOffset.value.value[0]!,r.y+nh+(r.height-nh)/2-sn*dx+c*dy+turnOffset.value.value[1]!];
 });
});
const cardX=computed(()=>card.value.value[1]!),cardY=computed(()=>card.value.value[2]!),cardHeight=computed(()=>Math.max(80*scale.value,card.value.value[3]!));
const bubble=computed(()=>l.value?bubbleShape(cardX.value,cardY.value,m.cardWidth*scale.value,cardHeight.value,l.value.edge,card.value.value[4]!,scale.value):[]);
function target(){
 const layout=l.value,slot=pointer.value?.providerId?rows.value.findIndex(p=>p.providerId===pointer.value?.providerId):pointer.value?.slot;if(!layout)return;
 if(pointer.value?.pressed){card.jump([0,cardX.value,cardY.value,cardHeight.value,card.value.value[4]!]);retainedProvider.value=null;return;}
 if(slot!==null&&slot!==undefined&&rows.value[slot]&&pointer.value?.expanded&&!pointer.value.pressed){
  const switching=retainedProvider.value!==rows.value[slot]!.providerId;const r=layout.rail,s=scale.value,ring=ringPositions.value[slot]!;
  const w=m.cardWidth*s,gap=(m.cardGap+m.pointerWidth)*s,pad=m.windowPadding*s;
  const available=layout.edge==='top'?layout.height-r.y-r.height-gap-pad:layout.edge==='bottom'?r.y-gap-pad:layout.height-2*pad;
  const h=Math.min(measured.value*s,Math.max(80*s,available));
  let x=ring[0]-w/2,y=ring[1]-h/2;
  if(layout.edge==='left')x=r.x+r.width+gap;else if(layout.edge==='right')x=r.x-gap-w;else if(layout.edge==='top')y=r.y+r.height+gap;else y=r.y-gap-h;
  x=clamp(x,pad,layout.width-w-pad);y=clamp(y,pad,layout.height-h-pad);
  const anchor=layout.metrics.horizontal?ring[0]:ring[1];
  if(retainedProvider.value===null)card.jump([0,x,y,h,anchor]);retainedProvider.value=rows.value[slot]!.providerId;
  if(switching){contentFade.jump([.35]);contentFade.to([1]);}card.to([1,x,y,h,anchor]);
 }else card.to([0,cardX.value,cardY.value,cardHeight.value,card.value.value[4]!]);
}
watch(()=>[pointer.value?.expanded,l.value?.metrics,pointer.value?.dock],()=>{
 if(!l.value)return;const q=l.value.metrics;const values=[pointer.value?.expanded?1:0,q.length,q.thickness,q.padding,q.pitch,q.round?1:0,pointer.value?.dock==='floating'?1:0];
 if(!pointer.value?.generation)rail.jump(values);else rail.to(values);
},{deep:true,immediate:true});
watch([()=>pointer.value?.providerId,()=>pointer.value?.slot,()=>pointer.value?.expanded,()=>pointer.value?.pressed,l,measured,rows],target);
watch(reveal,v=>{if(v===0&&pointer.value?.slot==null)retainedProvider.value=null;});
useResizeObserver(content,entries=>{const el=entries[0]?.target;if(el)measured.value=(el as HTMLElement).offsetHeight;});
const popover=ref<Point[]>([]);
watch(active,()=>popover.value=[]);
const geometry=computed<PanelGeometry>(()=>{
 const visible=reveal.value>.015&&active.value,layout=l.value,s=scale.value;
 let corridor:Point[]=[];
 if(visible&&layout){const r=layout.rail,a=card.value.value[4]!,g=m.cardGap*s;
  if(layout.edge==='right')corridor=[[r.x-g,a-12*s],[r.x,a-20*s],[r.x,a+20*s],[r.x-g,a+12*s]];
  if(layout.edge==='left')corridor=[[r.x+r.width,a-20*s],[r.x+r.width+g,a-12*s],[r.x+r.width+g,a+12*s],[r.x+r.width,a+20*s]];
  if(layout.edge==='top')corridor=[[a-20*s,r.y+r.height],[a+20*s,r.y+r.height],[a+12*s,r.y+r.height+g],[a-12*s,r.y+r.height+g]];
  if(layout.edge==='bottom')corridor=[[a-12*s,r.y-g],[a+12*s,r.y-g],[a+20*s,r.y],[a-20*s,r.y]];
 }
 const c=pageRect.value;const controls:Point[]=pages.value>1&&openness.value>.9?[[c.x,c.y],[c.x+c.width,c.y],[c.x+c.width,c.y+c.height],[c.x,c.y+c.height]]:[];
 let detail=visible?bubble.value:[];
 if(visible && popover.value.length){
  const points=[...detail,...popover.value], xs=points.map(p=>p[0]),ys=points.map(p=>p[1]);
  const left=Math.min(...xs),right=Math.max(...xs),top=Math.min(...ys),bottom=Math.max(...ys);
  detail=[[left,top],[right,top],[right,bottom],[left,bottom]];
 }
 return {controls,width:width.value,height:height.value,generation:pointer.value?.generation,display:pointer.value?.display,dock:pointer.value?.dock,rail:railPoints.value,detail,rings:openness.value>.9?ringPositions.value.map(([x,y],i)=>({x,y,radius:(m.ringDiameter+m.ringStroke)/2*s,slot:i,providerId:rows.value[i]!.providerId})):[],corridor};
});
let off:(()=>void)|undefined,stopped=false,sending=false,pending=false;
async function publish(){if(!isDesktop||stopped||!l.value)return;pending=true;if(sending)return;sending=true;try{while(pending&&!stopped){pending=false;await nextTick();await api('usage-panel/geometry','POST',geometry.value);}}finally{sending=false;}}
watch(geometry,value=>{emit('geometry',value);void publish();},{flush:'post',immediate:true});
onMounted(async()=>{if(isDesktop){const {listen}=await import('@tauri-apps/api/event');const dispose=await listen<PanelState>('usage-panel:pointer',e=>nativePointer.value=e.payload);if(stopped)dispose();else{off=dispose;await api('usage-panel/ready','POST');}}});
onUnmounted(()=>{stopped=true;off?.();});
// Native physical down/up is consumed by NSPanel; this is the AX/keyboard action.
async function open(providerId:string){if(pointer.value?.pressed)return;if(isDesktop)await api('subscriptions/open','POST',{providerId});else emit('open',providerId);}
const activityTrack=computed(()=>{
 const layout=l.value;if(!layout)return '';
 const r=layout.rail,s=scale.value,length=m.collapsedLength*s;
 if(layout.notch){const y=r.y+layout.notch.height+s;return `M ${r.x+(r.width-layout.notch.width)/2+12*s} ${y} h ${Math.max(0,layout.notch.width-24*s)}`;}
 const x=layout.edge==='left'?r.x+3*s:layout.edge==='right'?r.x+r.width-3*s:r.x+r.width/2;
 const y=layout.edge==='top'?r.y+3*s:layout.edge==='bottom'?r.y+r.height-3*s:r.y+r.height/2;
 return layout.metrics.horizontal?`M ${x-length/2+3*s} ${y} h ${length-6*s}`:`M ${x} ${y-length/2+3*s} v ${length-6*s}`;
});
</script>
<template><div class="usage-surface" @contextmenu.prevent><svg class="surface-shapes" :viewBox="`0 0 ${width} ${height}`" aria-hidden="true"><path class="rail-hit-target" :d="path(railPoints)" :fill="railFill"/><rect v-if="l?.notch && openness<.99 && alert!=='#000'" :x="l.rail.x+(l.rail.width-l.notch.width)/2+12*scale" :y="l.rail.y+l.notch.height" :width="Math.max(0,l.notch.width-24*scale)" :height="2*scale" :rx="scale" :fill="alert" :opacity="1-openness"/><path v-if="working && openness<.99" class="rail-activity" :d="activityTrack" pathLength="100" :opacity="1-openness" :stroke-width="2*scale"/><path v-if="active" :d="path(bubble)" :opacity="reveal"/></svg><div class="ring-layer" :style="{clipPath:`path('${path(railPoints)}')`}"><div v-for="(p,i) in rows" :key="p.providerId" class="ring-group" :style="{left:`${(ringPositions[i]?.[0]??0)-20*scale}px`,top:`${(ringPositions[i]?.[1]??0)-18*scale}px`,transform:`scale(${scale})`,opacity:openness,visibility:openness<.01?'hidden':'visible'}"><button class="ring-button" :data-provider-id="p.providerId" :style="{pointerEvents:openness>.9?'auto':'none'}" :aria-label="`${p.name} · ${t('usageOpenDetails')}`" @click="open(p.providerId)"><QuotaRing :provider="p" :warning-at="prefs?.warningAt" :show-percentage="l?.metrics.percentages"/></button></div><button v-if="pages>1 && openness>.9" class="rail-page" :aria-label="t('usageNextPage')" :style="{left:`${pageRect.x}px`,top:`${pageRect.y}px`,width:`${pageRect.width}px`,height:`${pageRect.height}px`,fontSize:`${11*scale}px`}" @click="nextPage">{{page+1}}/{{pages}} ›</button></div><aside v-if="active" class="bubble" :style="{left:`${cardX}px`,top:`${cardY}px`,height:`${cardHeight/scale}px`,transform:`scale(${scale})`,opacity:reveal,pointerEvents:reveal>.015?'auto':'none'}"><div ref="content" :style="{opacity:contentFade.value.value[0]}"><QuotaBubble :detail-placement="l?.edge==='right'?'left':'right'" @popover="popover=$event" :provider="active" :warning-at="prefs?.warningAt"/></div></aside></div></template>
<style scoped>
.rail-activity{fill:none;stroke:#f5f5f7;stroke-linecap:round;stroke-dasharray:33.333 100;animation:rail-working 1.8s linear infinite;pointer-events:none}
@keyframes rail-working{from{stroke-dashoffset:33.333}to{stroke-dashoffset:-100}}
@media(prefers-reduced-motion:reduce){.rail-activity{animation:none;stroke-dashoffset:0}}

.usage-surface{position:fixed;inset:0;color:#f5f5f7;pointer-events:none;user-select:none;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;line-height:normal;color-scheme:dark}.surface-shapes{position:absolute;inset:0;width:100%;height:100%;fill:#000;pointer-events:none}.rail-hit-target{pointer-events:fill;cursor:grab;touch-action:none}.rail-page{box-sizing:border-box;line-height:normal;transition:none;position:absolute;background:none;border:0;color:#ddd;pointer-events:auto;cursor:pointer;padding:0}.rail-page:hover,.rail-page:active{background:transparent;transform:none}.ring-layer{position:absolute;inset:0}.ring-group{position:absolute;width:40px;height:58px;transform-origin:top left}.ring-button{box-sizing:border-box;display:block;touch-action:none;line-height:normal;transition:none;vertical-align:baseline;border:0;border-radius:50%;background:transparent;color:inherit;width:40px;height:40px;margin:-2px 0 0;padding:2px 0 0;cursor:grab}.ring-button:hover,.ring-button:active{background:transparent;transform:none}.ring-button:active{cursor:grabbing}.ring-button:focus-visible{outline:2px solid #fff;outline-offset:3px}.ring-button .quota-ring{pointer-events:none}.bubble{position:absolute;width:250px;overflow:auto;transform-origin:top left;border-radius:20px}.bubble>div{width:250px}
</style>
