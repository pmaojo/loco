interface LoadingStateProps {
  message?: string;
}

export const LoadingState = ({ message = 'Loading graph…' }: LoadingStateProps) => (
  <div className="flex items-center justify-center rounded-xl border border-slate-800 bg-slate-900/40 p-6 text-slate-400">
    <span className="animate-pulse">{message}</span>
  </div>
);
