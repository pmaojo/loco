import classNames from 'classnames';
import { CommandHistoryEntry } from '../core/models/CommandConsole';

export interface CommandHistoryProps {
  history: CommandHistoryEntry[];
  onRefreshJob?: (jobId: string) => void;
  jobLoading?: boolean;
}

const statusClasses: Record<CommandHistoryEntry['status'], string> = {
  success: 'text-emerald-400',
  error: 'text-rose-400',
  running: 'text-amber-300',
};

const formatTime = (timestamp?: number): string => {
  if (!timestamp) {
    return '—';
  }
  return new Date(timestamp).toLocaleTimeString();
};

const renderStdout = (stdout: unknown): string => {
  if (stdout === undefined || stdout === null) {
    return '';
  }
  if (typeof stdout === 'string') {
    return stdout;
  }
  try {
    return JSON.stringify(stdout, null, 2);
  } catch (error) {
    return String(stdout);
  }
};

export const CommandHistory = ({ history, onRefreshJob, jobLoading }: CommandHistoryProps) => {
  if (history.length === 0) {
    return (
      <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-4 text-sm text-slate-400">
        No commands executed yet.
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {history.map((entry) => {
        const statusClass = statusClasses[entry.status];
        const stdout = renderStdout(entry.stdout);
        const stderr = entry.stderr ?? '';
        const canRefreshJob = entry.kind === 'job' && entry.context?.jobId && onRefreshJob;
        return (
          <article
            key={entry.id}
            className="rounded-xl border border-slate-800 bg-slate-950/60 p-4 text-sm text-slate-200"
            data-testid={`history-${entry.id}`}
          >
            <header className="mb-2 flex flex-wrap items-center justify-between gap-2">
              <div className="space-y-1">
                <p className="font-mono text-xs text-slate-400">{entry.command}</p>
                <p className="text-[11px] text-slate-500">
                  Started: {formatTime(entry.startedAt)}
                  {entry.completedAt ? ` · Completed: ${formatTime(entry.completedAt)}` : ''}
                  {typeof entry.exitCode === 'number' ? ` · Exit code: ${entry.exitCode}` : ''}
                  {entry.environment ? ` · Environment: ${entry.environment}` : ''}
                </p>
              </div>
              <div className="flex items-center gap-3">
                {canRefreshJob ? (
                  <button
                    type="button"
                    onClick={() => onRefreshJob(entry.context?.jobId as string)}
                    disabled={jobLoading}
                    className={classNames(
                      'rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300 transition hover:border-slate-500 hover:text-slate-100',
                      { 'opacity-50': jobLoading }
                    )}
                  >
                    Refresh status
                  </button>
                ) : null}
                <span className={classNames('text-xs font-semibold uppercase', statusClass)}>{entry.status}</span>
              </div>
            </header>

            {stdout ? (
              <section className="mb-2">
                <h3 className="mb-1 text-xs font-semibold text-slate-300">Output</h3>
                <pre className="max-h-64 overflow-auto rounded-lg bg-slate-900/80 p-3 text-xs text-slate-100">{stdout}</pre>
              </section>
            ) : null}

            {stderr ? (
              <section className="mb-2">
                <h3 className="mb-1 text-xs font-semibold text-rose-300">Errors</h3>
                <pre className="max-h-64 overflow-auto rounded-lg bg-rose-950/40 p-3 text-xs text-rose-200">{stderr}</pre>
              </section>
            ) : null}

            {entry.errorMessage ? (
              <p className="text-xs text-rose-400">{entry.errorMessage}</p>
            ) : null}
          </article>
        );
      })}
    </div>
  );
};
