import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from "recharts";
import { useId } from "react";

export function StackedBarChart({data, series, valueLabel, label, highlighted}: {
  data: Record<string, string | number>[];
  series: {key:string; name:string; color:string}[];
  valueLabel: (value:number, compact?:boolean) => string;
  label: string;
  highlighted?: string;
}) {
  const id = useId();
  return <div className="stacked-bar-chart" role="group" aria-label={label}>
    <ResponsiveContainer width="100%" height={260} minWidth={0} debounce={50}>
      <BarChart data={data} accessibilityLayer margin={{top:12,right:8,left:0,bottom:0}} maxBarSize={48}>
        <CartesianGrid stroke="var(--border)" strokeDasharray="3 4" vertical={false} />
        <XAxis dataKey="label" tickLine={false} axisLine={false} tick={{fill:"var(--text-secondary)",fontSize:11}} minTickGap={22} />
        <YAxis tickFormatter={n => valueLabel(n,true)} tickLine={false} axisLine={false} tick={{fill:"var(--text-secondary)",fontSize:11}} width={68} />
        <Tooltip cursor={{fill:"var(--surface-subtle)"}} contentStyle={{background:"var(--surface)",border:"1px solid var(--border)",borderRadius:10,color:"var(--text)",fontSize:12}}
          formatter={(value) => valueLabel(Number(value))} />
        {series.map(series => <Bar key={series.key} id={`${id}-${series.key}`} name={series.name} dataKey={series.key} stackId="usage" fill={series.color}
          fillOpacity={!highlighted || highlighted === series.key ? 1 : .25} isAnimationActive={false} />)}
      </BarChart>
    </ResponsiveContainer>
  </div>;
}
