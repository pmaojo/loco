import { ComponentFilterMap, ComponentStatistics } from '../core/models/Graph';

interface ControlsPanelProps {
  filters: ComponentFilterMap;
  onToggle: (type: keyof ComponentFilterMap) => void;
  onReload: () => void;
  loading: boolean;
  statistics?: ComponentStatistics;
}

const LABELS: Record<keyof ComponentFilterMap, string> = {
  route: 'Routes',
  background_worker: 'Background Workers',
  scheduler_job: 'Scheduler Jobs',
  task: 'Tasks',
};

export const ControlsPanel = ({
  filters,
  onToggle,
  onReload,
  loading,
  statistics,
}: ControlsPanelProps) => {
  return (
    <div className="flex flex-col gap-4 rounded-xl border border-slate-800 bg-slate-900/60 p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Graph Controls</h2>
        <button
          type="button"
          onClick={onReload}
          className="rounded-lg bg-sky-500 px-3 py-1 text-sm font-semibold text-slate-950 transition hover:bg-sky-400 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={loading}
        >
          {loading ? 'Refreshing…' : 'Reload Data'}
        </button>
      </div>
      <div className="grid grid-cols-1 gap-3 text-sm">
        {(Object.keys(filters) as (keyof ComponentFilterMap)[]).map((key) => (
          <label key={key} className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={filters[key]}
              onChange={() => onToggle(key)}
              className="h-4 w-4 rounded border-slate-700 bg-slate-900 text-sky-500 focus:ring-sky-500"
            />
            <span className="flex-1">
              {LABELS[key]}
              {statistics ? (
                <span className="ml-2 rounded-full bg-slate-800 px-2 py-0.5 text-xs text-slate-300">
                  {statistics[key] ?? 0}
                </span>
              ) : null}
            </span>
          </label>
        ))}
      </div>
    </div>
  );
};
