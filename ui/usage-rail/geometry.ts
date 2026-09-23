import m from '../../shared/usage-panel.json';
export type Point = [number, number];
export const clamp = (v:number,a:number,b:number)=>Math.max(a,Math.min(b,v));
function curve(points:Point[],a:Point,b:Point,c:Point){
 const p=points[points.length-1]!;
 for(let i=1;i<=16;i++){const t=i/16,u=1-t;points.push([u*u*u*p[0]+3*u*u*t*a[0]+3*u*t*t*b[0]+t*t*t*c[0],u*u*u*p[1]+3*u*u*t*a[1]+3*u*t*t*b[1]+t*t*t*c[1]]);}
}
function corner(p:Point[],cx:number,cy:number,r:number,start:number,exponent=4){
 for(let i=1;i<=48;i++){const t=start+i/48*Math.PI/2;const c=Math.cos(t),s=Math.sin(t);p.push([cx+r*Math.sign(c)*Math.pow(Math.abs(c),2/exponent),cy+r*Math.sign(s)*Math.pow(Math.abs(s),2/exponent)]);}
}
// Canonical right-facing rail: transform only the silhouette, never its contents.
export function berth(o:number,length:number,thickness:number,round:number,floating:number,scale:number):Point[]{
 const h=m.collapsedLength*scale+(length-m.collapsedLength*scale)*o;
 const w=m.sliverWidth*scale+(thickness-m.sliverWidth*scale)*o;
 const f=(m.flareHeight+(32-m.flareHeight)*round)*scale*o*(1-floating);
 const r=Math.min((m.sliverWidth+(m.cornerRadius+(32-m.cornerRadius)*round-m.sliverWidth)*o)*scale,w,(h-2*f)/2);
 const fw=Math.max(0,Math.min((m.flareWidth+(32-m.flareWidth)*round)*scale*o*(1-floating),w-r));
 const p:Point[]=[[r,f],[w-fw,f]];
 curve(p,[w-fw*.45,f],[w,f*.55],[w,0]);p.push([w,h]);
 curve(p,[w,h-f*.55],[w-fw*.45,h-f],[w-fw,h-f]);p.push([r,h-f]);
 corner(p,r,h-f-r,r,Math.PI/2,4-2*round);p.push([0,f+r]);corner(p,r,f+r,r,Math.PI,4-2*round);
 // Floating is a true capsule, including the formerly screen-facing corners.
 if(floating>0){const radius=Math.min(w/2,h/2);
  // Equal perimeter sampling avoids a topology jump while undocking.
  const resample=(poly:Point[],n:number)=>{const loop=[...poly,poly[0]!],dist=[0];for(let i=1;i<loop.length;i++)dist.push(dist[i-1]!+Math.hypot(loop[i]![0]-loop[i-1]![0],loop[i]![1]-loop[i-1]![1]));const out:Point[]=[];let k=1;for(let i=0;i<n;i++){const d=dist.at(-1)!*i/n;while(k<dist.length-1&&dist[k]!<d)k++;const a=loop[k-1]!,b=loop[k]!,v=(d-dist[k-1]!)/(dist[k]!-dist[k-1]!||1);out.push([a[0]+(b[0]-a[0])*v,a[1]+(b[1]-a[1])*v]);}return out;};
  // Start both contours at the inner top tangent, walk clockwise.
  const cap:Point[]=[[radius,0],[w-radius,0]];corner(cap,w-radius,radius,radius,-Math.PI/2,2);cap.push([w,h-radius]);corner(cap,w-radius,h-radius,radius,0,2);cap.push([radius,h]);corner(cap,radius,h-radius,radius,Math.PI/2,2);cap.push([0,radius]);corner(cap,radius,radius,radius,Math.PI,2);
  const a=resample(p,180),b=resample(cap,180);return a.map(([x,y],i)=>[x+(b[i]![0]-x)*floating,y+(b[i]![1]-y)*floating]);
 }
 return p;
}
export function notchOutline(o:number,width:number,housingWidth:number,housingHeight:number,thickness:number,round:number,scale:number):Point[]{
 if(o<=0)return [];
 const w=housingWidth+(width-housingWidth)*o,h=housingHeight+thickness*o;
 const fw=(m.flareWidth+(32-m.flareWidth)*round)*scale*o,fh=(m.flareHeight+(32-m.flareHeight)*round)*scale*o,r=Math.min((m.cornerRadius+(32-m.cornerRadius)*round)*scale,w/2,h-fh);
 const x=(width-w)/2,p:Point[]=[[x-fw,0]];
 curve(p,[x-fw*.45,0],[x,fh*.45],[x,fh]);p.push([x,h-r]);
 // Walk counter-clockwise along the lower housing continuation.
 for(let i=0;i<=32;i++){const t=Math.PI-i/32*Math.PI/2;p.push([x+r+r*Math.cos(t),h-r+r*Math.sin(t)]);}
 p.push([x+w-r,h]);for(let i=0;i<=32;i++){const t=Math.PI/2-i/32*Math.PI/2;p.push([x+w-r+r*Math.cos(t),h-r+r*Math.sin(t)]);}
 p.push([x+w,fh]);curve(p,[x+w,fh*.45],[x+w+fw*.45,0],[x+w+fw,0]);return p;
}
export function bubbleShape(x:number,y:number,w:number,h:number,side:'left'|'right'|'top'|'bottom',anchor:number,scale:number):Point[]{
 const vertical=side==='left'||side==='right';
 const r=Math.min(m.cardRadius*scale,w/2,h/2);
 const half=Math.max(0,Math.min(m.pointerHeight*scale,(vertical?h:w)/2-r));
 const tail=Math.min(m.pointerWidth*scale,half/2);
 const p:Point[]=[[x+r,y]];
 function flank(base:Point,tip:Point,end:Point){
  const vertical=side==='left'||side==='right';
  const axis:Point=vertical?[0,Math.sign(end[1]-base[1])]:[Math.sign(end[0]-base[0]),0];
  const normal:Point=vertical?[Math.sign(tip[0]-base[0]),0]:[0,Math.sign(tip[1]-base[1])];
  // Open the tip angle while keeping both roots tangent to the card.
  curve(p,[base[0]+axis[0]*half*.72,base[1]+axis[1]*half*.72],
   [tip[0]-normal[0]*tail*.42-axis[0]*half*.22,tip[1]-normal[1]*tail*.42-axis[1]*half*.22],tip);
  curve(p,[tip[0]-normal[0]*tail*.42+axis[0]*half*.22,tip[1]-normal[1]*tail*.42+axis[1]*half*.22],
   [end[0]-axis[0]*half*.72,end[1]-axis[1]*half*.72],end);
 }
 const a=clamp(anchor,(side==='left'||side==='right'?y:x)+r+half,(side==='left'||side==='right'?y+h:x+w)-r-half);
 if(side==='top'){p.push([a-half,y]);flank([a-half,y],[a,y-tail],[a+half,y]);}p.push([x+w-r,y]);corner(p,x+w-r,y+r,r,-Math.PI/2,2.3);
 if(side==='right'){p.push([x+w,a-half]);flank([x+w,a-half],[x+w+tail,a],[x+w,a+half]);}p.push([x+w,y+h-r]);corner(p,x+w-r,y+h-r,r,0,2.3);
 if(side==='bottom'){p.push([a+half,y+h]);flank([a+half,y+h],[a,y+h+tail],[a-half,y+h]);}p.push([x+r,y+h]);corner(p,x+r,y+h-r,r,Math.PI/2,2.3);
 if(side==='left'){p.push([x,a+half]);flank([x,a+half],[x-tail,a],[x,a-half]);}p.push([x,y+r]);corner(p,x+r,y+r,r,Math.PI,2.3);return p;
}
export const path=(p:Point[])=>p.length?`M${p.map(v=>v.join(',')).join('L')}Z`:'';
