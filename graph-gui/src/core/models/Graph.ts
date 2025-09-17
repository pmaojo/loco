export interface RawRouteDescriptor {
  path: string;
  methods: string[];
}

export interface RawBackgroundWorker {
  name: string;
  command: string;
  tags?: string[];
}

export interface RawSchedulerJob {
  name: string;
  command: string;
  schedule: string;
  run_on_start: boolean;
  shell: boolean;
  tags?: string[];
}

export interface RawTask {
  name: string;
  description?: string;
  tags?: string[];
}

export interface RawGraphSnapshot {
  routes: RawRouteDescriptor[];
  dependencies: {
    background_workers: RawBackgroundWorker[];
    scheduler_jobs: RawSchedulerJob[];
    tasks: RawTask[];
  };
  health: {
    ok: boolean;
    details?: string;
    [key: string]: unknown;
  };
}

export type ComponentType =
  | 'application'
  | 'route'
  | 'background_worker'
  | 'scheduler_job'
  | 'task';

export interface BaseNode<T extends ComponentType = ComponentType> {
  id: string;
  label: string;
  type: T;
}

export interface ApplicationNode extends BaseNode<'application'> {
  data: {
    healthy: boolean;
    description: string;
  };
}

export interface RouteNode extends BaseNode<'route'> {
  data: RawRouteDescriptor;
}

export interface BackgroundWorkerNode extends BaseNode<'background_worker'> {
  data: RawBackgroundWorker;
}

export interface SchedulerJobNode extends BaseNode<'scheduler_job'> {
  data: RawSchedulerJob;
}

export interface TaskNode extends BaseNode<'task'> {
  data: RawTask;
}

export type GraphNode =
  | ApplicationNode
  | RouteNode
  | BackgroundWorkerNode
  | SchedulerJobNode
  | TaskNode;

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  type: 'dependency' | 'flow';
}

export interface GraphViewModel {
  nodes: GraphNode[];
  edges: GraphEdge[];
  statistics: ComponentStatistics;
}

export type ComponentStatistics = Record<Exclude<ComponentType, 'application'>, number>;

export type ComponentFilterMap = Record<Exclude<ComponentType, 'application'>, boolean>;
