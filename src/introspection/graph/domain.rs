use std::collections::BTreeMap;

/// Represents the full application graph composed of nodes and edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Describes a vertex in the application graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub kind: ComponentKind,
}

/// Categorises the type of component represented by a [`GraphNode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentKind {
    Application {
        name: String,
    },
    HttpRoute {
        path: String,
        methods: Vec<String>,
    },
    BackgroundWorker {
        name: String,
        queue: Option<String>,
    },
    SchedulerJob {
        name: String,
        schedule: String,
        command: String,
        run_on_start: bool,
        shell: bool,
        tags: Vec<String>,
    },
    Task {
        name: String,
        detail: Option<String>,
    },
}

/// Relationship between two nodes in the application graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

/// Describes the semantics of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    Contains,
    Triggers,
}

impl EdgeKind {
    fn sort_key(self) -> u8 {
        match self {
            Self::Contains => 0,
            Self::Triggers => 1,
        }
    }
}

/// HTTP route description independent of the framework wiring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDescriptor {
    pub path: String,
    pub methods: Vec<String>,
}

/// Background worker description extracted from queue registries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundWorkerDescriptor {
    pub name: String,
    pub queue: Option<String>,
}

/// Scheduler job description extracted from the scheduler configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerJobDescriptor {
    pub name: String,
    pub schedule: String,
    pub command: String,
    pub run_on_start: bool,
    pub shell: bool,
    pub tags: Vec<String>,
}

/// Task description exposed by the task registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDescriptor {
    pub name: String,
    pub detail: Option<String>,
}

/// Repository abstraction for retrieving HTTP routes.
pub trait RoutesRepository {
    fn routes(&self) -> Vec<RouteDescriptor>;
}

/// Repository abstraction for retrieving background workers.
pub trait BackgroundWorkerRepository {
    fn workers(&self) -> Vec<BackgroundWorkerDescriptor>;
}

/// Repository abstraction for retrieving scheduler jobs.
pub trait SchedulerRepository {
    fn jobs(&self) -> Vec<SchedulerJobDescriptor>;
}

/// Repository abstraction for retrieving registered tasks.
pub trait TaskRepository {
    fn tasks(&self) -> Vec<TaskDescriptor>;
}

/// Builds an [`ApplicationGraph`] from the different repositories.
pub struct GraphBuilder<'a, R, B, S, T>
where
    R: RoutesRepository + ?Sized,
    B: BackgroundWorkerRepository + ?Sized,
    S: SchedulerRepository + ?Sized,
    T: TaskRepository + ?Sized,
{
    app_name: &'a str,
    routes: &'a R,
    background_workers: &'a B,
    scheduler: &'a S,
    tasks: &'a T,
}

impl<'a, R, B, S, T> GraphBuilder<'a, R, B, S, T>
where
    R: RoutesRepository + ?Sized,
    B: BackgroundWorkerRepository + ?Sized,
    S: SchedulerRepository + ?Sized,
    T: TaskRepository + ?Sized,
{
    /// Creates a new builder referencing the repositories needed to materialise the graph.
    pub fn new(
        app_name: &'a str,
        routes: &'a R,
        background_workers: &'a B,
        scheduler: &'a S,
        tasks: &'a T,
    ) -> Self {
        Self {
            app_name,
            routes,
            background_workers,
            scheduler,
            tasks,
        }
    }

    /// Materialises the graph by querying every repository.
    #[allow(clippy::too_many_lines)]
    pub fn build(&self) -> ApplicationGraph {
        let mut nodes: BTreeMap<String, GraphNode> = BTreeMap::new();
        let mut edges: Vec<GraphEdge> = Vec::new();

        let root_id = format!("app:{}", self.app_name);
        nodes.insert(
            root_id.clone(),
            GraphNode {
                id: root_id.clone(),
                kind: ComponentKind::Application {
                    name: self.app_name.to_owned(),
                },
            },
        );

        let mut task_nodes: BTreeMap<String, String> = BTreeMap::new();
        for task in self.tasks.tasks() {
            let TaskDescriptor { name, detail } = task;
            let node_id = format!("task:{name}");
            nodes.insert(
                node_id.clone(),
                GraphNode {
                    id: node_id.clone(),
                    kind: ComponentKind::Task {
                        name: name.clone(),
                        detail,
                    },
                },
            );
            task_nodes.insert(name, node_id.clone());
            edges.push(GraphEdge {
                from: root_id.clone(),
                to: node_id,
                kind: EdgeKind::Contains,
            });
        }

        for route in self.routes.routes() {
            let RouteDescriptor { path, mut methods } = route;
            methods.sort();
            methods.dedup();
            let node_id = format!("route:{path}");
            nodes.insert(
                node_id.clone(),
                GraphNode {
                    id: node_id.clone(),
                    kind: ComponentKind::HttpRoute { path, methods },
                },
            );
            edges.push(GraphEdge {
                from: root_id.clone(),
                to: node_id,
                kind: EdgeKind::Contains,
            });
        }

        for worker in self.background_workers.workers() {
            let BackgroundWorkerDescriptor { name, queue } = worker;
            let node_id = format!("worker:{name}");
            nodes.insert(
                node_id.clone(),
                GraphNode {
                    id: node_id.clone(),
                    kind: ComponentKind::BackgroundWorker { name, queue },
                },
            );
            edges.push(GraphEdge {
                from: root_id.clone(),
                to: node_id,
                kind: EdgeKind::Contains,
            });
        }

        let mut scheduler_edges: Vec<GraphEdge> = Vec::new();
        for job in self.scheduler.jobs() {
            let SchedulerJobDescriptor {
                name,
                schedule,
                command,
                run_on_start,
                shell,
                mut tags,
            } = job;

            let node_id = format!("scheduler:{name}");
            tags.sort();
            tags.dedup();

            let trigger = scheduler_task_reference(&command, shell);

            nodes.insert(
                node_id.clone(),
                GraphNode {
                    id: node_id.clone(),
                    kind: ComponentKind::SchedulerJob {
                        name,
                        schedule,
                        command,
                        run_on_start,
                        shell,
                        tags,
                    },
                },
            );
            edges.push(GraphEdge {
                from: root_id.clone(),
                to: node_id.clone(),
                kind: EdgeKind::Contains,
            });

            if let Some(task_name) = trigger {
                if let Some(task_node_id) = task_nodes.get(&task_name) {
                    scheduler_edges.push(GraphEdge {
                        from: node_id,
                        to: task_node_id.clone(),
                        kind: EdgeKind::Triggers,
                    });
                }
            }
        }

        edges.extend(scheduler_edges);
        edges.sort_by(|a, b| {
            let a_key = (&a.from, &a.to, a.kind.sort_key());
            let b_key = (&b.from, &b.to, b.kind.sort_key());
            a_key.cmp(&b_key)
        });
        edges.dedup();

        ApplicationGraph {
            nodes: nodes.into_values().collect(),
            edges,
        }
    }
}

fn scheduler_task_reference(command: &str, shell: bool) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = if shell {
        trimmed
            .strip_prefix("task ")
            .and_then(|rest| rest.split_whitespace().next())
    } else {
        trimmed.split_whitespace().next()
    }?;

    Some(candidate.to_string())
}
