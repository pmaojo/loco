import { ComponentStatistics } from '../core/models/Graph';

interface GraphLegendProps {
  statistics: ComponentStatistics;
}

const ITEMS: { type: keyof ComponentStatistics; label: string; color: string }[] = [
  { type: 'route', label: 'Routes', color: '#60a5fa' },
  { type: 'background_worker', label: 'Background Workers', color: '#f97316' },
  { type: 'scheduler_job', label: 'Scheduler Jobs', color: '#a855f7' },
  { type: 'task', label: 'Tasks', color: '#2dd4bf' },
];

export const GraphLegend = ({ statistics }: GraphLegendProps) => (
  <div className="flex flex-wrap gap-4 rounded-xl border border-slate-800 bg-slate-900/60 p-4 text-sm">
    {ITEMS.map((item) => (
      <span key={item.type} className="flex items-center gap-2">
        <span
          className="h-3 w-3 rounded-full"
          style={{ backgroundColor: item.color }}
          aria-hidden
        />
        <span className="text-slate-200">
          {item.label}
          <span className="ml-2 text-xs text-slate-400">{statistics[item.type] ?? 0}</span>
        </span>
      </span>
    ))}
  </div>
);
