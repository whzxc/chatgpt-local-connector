import { onUnmounted, shallowRef } from 'vue';
// Damped harmonic motion: response is the natural period, not a CSS duration.
// Retargeting retains the current position AND velocity. No frames at rest.
export function useSpring(initial:number[],response:number,damping:number,onFrame?:()=>void) {
 const value=shallowRef([...initial]);let goal=[...initial],velocity=initial.map(()=>0),frame=0,last=0;
 const reduced=matchMedia('(prefers-reduced-motion: reduce)');
 function step(now:number){
  const dt=Math.min((now-last)/1000,.032);last=now;
  const w=2*Math.PI/response,wd=w*Math.sqrt(1-damping*damping),e=Math.exp(-damping*w*dt);
  let settled=true;
  value.value=value.value.map((x,i)=>{const target=goal[i]!,v=velocity[i]!,y=x-target,b=(v+damping*w*y)/wd,c=Math.cos(wd*dt),s=Math.sin(wd*dt);const next=e*(y*c+b*s),speed=e*((-damping*w)*(y*c+b*s)+wd*(-y*s+b*c));velocity[i]=speed;if(Math.abs(next)>.001||Math.abs(speed)>.005)settled=false;return target+next;});
  if(settled){value.value=[...goal];velocity.fill(0);frame=0;}else frame=requestAnimationFrame(step);
  onFrame?.();
 }
 function to(next:number[]){goal=[...next];if(reduced.matches){cancelAnimationFrame(frame);frame=0;velocity.fill(0);value.value=[...goal];onFrame?.();return;}if(!frame){last=performance.now();frame=requestAnimationFrame(step);}}
 function jump(next:number[]){cancelAnimationFrame(frame);frame=0;goal=[...next];velocity=next.map(()=>0);value.value=[...next];onFrame?.();}
 const reduce=()=>{if(reduced.matches)jump(goal);};reduced.addEventListener('change',reduce);
 onUnmounted(()=>{cancelAnimationFrame(frame);reduced.removeEventListener('change',reduce);});
 return {value,to,jump};
}
