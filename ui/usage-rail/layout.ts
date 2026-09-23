export type Edge='left'|'right'|'top'|'bottom';
export type Dock=Edge|'floating';
export interface PanelPreferences {visible?:boolean;autoCollapse:boolean;notchFusion:boolean;size:'small'|'standard'|'large';spacing:'compact'|'standard'|'roomy';horizontalPercentages:boolean;alertColor:boolean;warningAt:number;placement:{dock:Dock;display:string;x:number;y:number}}
export interface PanelLayout {screenX?:number;screenY?:number;width:number;height:number;rail:{x:number;y:number;width:number;height:number};edge:Edge;notch?:{width:number;height:number};metrics:{visibleCount?:number;footer?:number;scale:number;length:number;thickness:number;padding:number;pitch:number;item:number;percentages:boolean;round:boolean;horizontal:boolean}}
export interface PanelState {expanded:boolean;providerId?:string|null;slot:number|null;generation:number;display:string;dock:Dock;pressed:boolean;layout:PanelLayout|null;preferences:PanelPreferences}

export interface PanelGeometry {
 controls: [number,number][]; width:number; height:number;
 generation?:number; display?:string; dock?:Dock;
 rail:[number,number][]; detail:[number,number][]; corridor:[number,number][];
 rings:{x:number;y:number;radius:number;slot:number;providerId:string}[];
}
