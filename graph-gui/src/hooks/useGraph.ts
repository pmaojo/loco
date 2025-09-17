import { useCallback, useEffect, useMemo, useState } from 'react';
import { AssistantInsight, AssistantPort } from '../core/models/Assistant';
import {
  ComponentFilterMap,
  GraphEdge,
  GraphNode,
  GraphViewModel,
} from '../core/models/Graph';
import { GraphService } from '../core/services/GraphService';

const defaultFilters: ComponentFilterMap = {
  route: true,
  background_worker: true,
  scheduler_job: true,
  task: true,
};

type GraphStatus = 'idle' | 'loading' | 'ready' | 'error';
type AssistantStatus = 'idle' | 'loading' | 'ready' | 'error';

export interface AssistantState {
  status: AssistantStatus;
  insight?: AssistantInsight;
  error?: string;
}

export interface UseGraphResult {
  status: GraphStatus;
  error?: string;
  graph?: GraphViewModel;
  filteredNodes: GraphNode[];
  filteredEdges: GraphEdge[];
  filters: ComponentFilterMap;
  toggleFilter: (type: keyof ComponentFilterMap) => void;
  selectNode: (node: GraphNode | null) => void;
  selectedNode: GraphNode | null;
  assistant: AssistantState;
  requestInsight: (node: GraphNode, prompt?: string) => Promise<void>;
  reload: () => Promise<void>;
}

export const useGraph = (
  graphService: GraphService,
  assistantService: AssistantPort
): UseGraphResult => {
  const [status, setStatus] = useState<GraphStatus>('idle');
  const [graph, setGraph] = useState<GraphViewModel>();
  const [error, setError] = useState<string>();
  const [filters, setFilters] = useState<ComponentFilterMap>(defaultFilters);
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [assistant, setAssistant] = useState<AssistantState>({ status: 'idle' });

  const load = useCallback(async () => {
    setStatus('loading');
    setError(undefined);
    try {
      const viewModel = await graphService.load();
      setGraph(viewModel);
      setStatus('ready');
    } catch (err) {
      setStatus('error');
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  }, [graphService]);

  useEffect(() => {
    void load();
  }, [load]);

  const filteredNodes = useMemo(() => {
    if (!graph) {
      return [] as GraphNode[];
    }

    return graph.nodes.filter((node) => {
      if (node.type === 'application') {
        return true;
      }

      return filters[node.type];
    });
  }, [filters, graph]);

  const filteredEdges = useMemo(() => {
    if (!graph) {
      return [] as GraphEdge[];
    }

    const allowedNodes = new Set(filteredNodes.map((node) => node.id));
    return graph.edges.filter(
      (edge) => allowedNodes.has(edge.source) && allowedNodes.has(edge.target)
    );
  }, [filteredNodes, graph]);

  useEffect(() => {
    if (!selectedNode) {
      return;
    }

    const stillVisible = filteredNodes.some((node) => node.id === selectedNode.id);
    if (!stillVisible) {
      setSelectedNode(null);
      setAssistant({ status: 'idle' });
    }
  }, [filteredNodes, selectedNode]);

  const toggleFilter = useCallback((type: keyof ComponentFilterMap) => {
    setFilters((prev) => ({ ...prev, [type]: !prev[type] }));
  }, []);

  const selectNode = useCallback((node: GraphNode | null) => {
    setSelectedNode(node);
    setAssistant({ status: 'idle' });
  }, []);

  const requestInsight = useCallback(
    async (node: GraphNode, prompt?: string) => {
      setAssistant({ status: 'loading' });
      try {
        const insight = await assistantService.explainNode(node, { prompt });
        setAssistant({ status: 'ready', insight });
      } catch (err) {
        setAssistant({
          status: 'error',
          error: err instanceof Error ? err.message : 'Unknown error',
        });
      }
    },
    [assistantService]
  );

  return {
    status,
    error,
    graph,
    filteredNodes,
    filteredEdges,
    filters,
    toggleFilter,
    selectNode,
    selectedNode,
    assistant,
    requestInsight,
    reload: load,
  };
};
