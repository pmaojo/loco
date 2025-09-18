import { useMemo } from 'react';
import { HttpGraphRepository } from './adapters/http/HttpGraphRepository';
import { HttpAssistantService } from './adapters/http/HttpAssistantService';
import { GraphService } from './core/services/GraphService';
import { CommandConsoleService } from './core/services/CommandConsoleService';
import { useGraph } from './hooks/useGraph';
import { GraphCanvas } from './components/GraphCanvas';
import { ControlsPanel } from './components/ControlsPanel';
import { NodeDetails } from './components/NodeDetails';
import { GraphLegend } from './components/GraphLegend';
import { LoadingState } from './components/LoadingState';
import { CommandConsole } from './components/CommandConsole';
import { HttpCliService } from './adapters/http/HttpCliService';

const App = () => {
  const repository = useMemo(() => new HttpGraphRepository(), []);
  const assistantService = useMemo(() => new HttpAssistantService(), []);
  const graphService = useMemo(() => new GraphService(repository), [repository]);
  const cliAdapter = useMemo(() => new HttpCliService(), []);
  const commandConsoleService = useMemo(() => new CommandConsoleService(cliAdapter), [cliAdapter]);

  const {
    status,
    error,
    graph,
    filteredNodes,
    filteredEdges,
    filters,
    toggleFilter,
    reload,
    selectedNode,
    selectNode,
    assistant,
    requestInsight,
  } = useGraph(graphService, assistantService);

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100">
      <main className="mx-auto flex max-w-7xl flex-col gap-6 px-6 py-8">
        <header className="flex flex-col gap-2">
          <h1 className="text-3xl font-bold">Loco Application Graph</h1>
          <p className="max-w-2xl text-sm text-slate-400">
            Explore the relationships between routes, workers and scheduled jobs. Use the AI assistant to
            collect remediation tips when components misbehave.
          </p>
        </header>

        {graph ? <GraphLegend statistics={graph.statistics} /> : null}

        <div className="grid gap-6 lg:grid-cols-[2fr_1fr]">
          <section className="space-y-4">
            {status === 'loading' && <LoadingState />}
            {status === 'error' && (
              <div className="rounded-xl border border-rose-700 bg-rose-950/40 p-6 text-sm text-rose-300">
                <p className="mb-4">Unable to load the graph: {error}</p>
                <button
                  type="button"
                  onClick={() => reload()}
                  className="rounded-lg bg-rose-500 px-3 py-1 font-semibold text-rose-950 transition hover:bg-rose-400"
                >
                  Try again
                </button>
              </div>
            )}
            {status === 'ready' && filteredNodes.length > 0 && (
              <GraphCanvas
                nodes={filteredNodes}
                edges={filteredEdges}
                onNodeSelect={selectNode}
                selectedNode={selectedNode}
              />
            )}
            {status === 'ready' && filteredNodes.length === 0 && (
              <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-6 text-sm text-slate-400">
                No components match the current filters.
              </div>
            )}
          </section>

          <aside className="flex flex-col gap-4">
            <ControlsPanel
              filters={filters}
              onToggle={toggleFilter}
              onReload={() => reload()}
              loading={status === 'loading'}
              statistics={graph?.statistics}
            />
            <NodeDetails
              node={selectedNode}
              assistant={assistant}
              onRequestInsight={requestInsight}
            />
          </aside>
        </div>

        <CommandConsole service={commandConsoleService} />
      </main>
    </div>
  );
};

export default App;
