import {
  ComponentStatistics,
  GraphEdge,
  GraphNode,
  GraphViewModel,
  RawGraphSnapshot,
} from '../models/Graph';
import { GraphRepository } from '../ports/GraphRepository';

export const ROOT_NODE_ID = 'application-root';

export class GraphService {
  constructor(private readonly repository: GraphRepository) {}

  async load(): Promise<GraphViewModel> {
    const snapshot = await this.repository.fetchGraph();
    return GraphService.toViewModel(snapshot);
  }

  static toViewModel(snapshot: RawGraphSnapshot): GraphViewModel {
    const root = buildRootNode(snapshot);
    const routes = snapshot.routes.map((route, index): GraphNode => ({
      id: `route:${index}:${route.path}`,
      label: route.path,
      type: 'route',
      data: route,
    }));
    const workers = snapshot.dependencies.background_workers.map((worker, index): GraphNode => ({
      id: `background_worker:${index}:${worker.name}`,
      label: worker.name,
      type: 'background_worker',
      data: worker,
    }));
    const jobs = snapshot.dependencies.scheduler_jobs.map((job, index): GraphNode => ({
      id: `scheduler_job:${index}:${job.name}`,
      label: job.name,
      type: 'scheduler_job',
      data: job,
    }));
    const tasks = snapshot.dependencies.tasks.map((task, index): GraphNode => ({
      id: `task:${index}:${task.name}`,
      label: task.name,
      type: 'task',
      data: task,
    }));

    const nodes: GraphNode[] = [root, ...routes, ...workers, ...jobs, ...tasks];
    const edges: GraphEdge[] = nodes
      .filter((node) => node.type !== 'application')
      .map((node) => ({
        id: `edge:${ROOT_NODE_ID}->${node.id}`,
        source: ROOT_NODE_ID,
        target: node.id,
        type: 'dependency',
      }));

    const statistics: ComponentStatistics = {
      route: routes.length,
      background_worker: workers.length,
      scheduler_job: jobs.length,
      task: tasks.length,
    };

    return { nodes, edges, statistics };
  }
}

function buildRootNode(snapshot: RawGraphSnapshot): GraphNode {
  const description = snapshot.health.ok
    ? 'Application components are healthy.'
    : 'Application reported issues. Inspect individual nodes for details.';
  return {
    id: ROOT_NODE_ID,
    label: 'Application',
    type: 'application',
    data: {
      healthy: snapshot.health.ok,
      description,
    },
  };
}
