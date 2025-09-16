use loco_rs::introspection::graph::domain::{
    ApplicationGraph, BackgroundWorkerDescriptor, BackgroundWorkerRepository, ComponentKind,
    EdgeKind, GraphBuilder, GraphEdge, GraphNode, RouteDescriptor, RoutesRepository,
    SchedulerJobDescriptor, SchedulerRepository, TaskDescriptor, TaskRepository,
};

struct RoutesStub {
    routes: Vec<RouteDescriptor>,
}

impl RoutesRepository for RoutesStub {
    fn routes(&self) -> Vec<RouteDescriptor> {
        self.routes.clone()
    }
}

struct WorkersStub {
    workers: Vec<BackgroundWorkerDescriptor>,
}

impl BackgroundWorkerRepository for WorkersStub {
    fn workers(&self) -> Vec<BackgroundWorkerDescriptor> {
        self.workers.clone()
    }
}

struct SchedulerStub {
    jobs: Vec<SchedulerJobDescriptor>,
}

impl SchedulerRepository for SchedulerStub {
    fn jobs(&self) -> Vec<SchedulerJobDescriptor> {
        self.jobs.clone()
    }
}

struct TasksStub {
    tasks: Vec<TaskDescriptor>,
}

impl TaskRepository for TasksStub {
    fn tasks(&self) -> Vec<TaskDescriptor> {
        self.tasks.clone()
    }
}

fn find_node<'a>(
    graph: &'a ApplicationGraph,
    predicate: impl Fn(&'a GraphNode) -> bool,
) -> &'a GraphNode {
    graph
        .nodes
        .iter()
        .find(|node| predicate(node))
        .expect("expected node to exist")
}

#[test]
fn builds_graph_with_routes_workers_jobs_and_tasks() {
    let routes = RoutesStub {
        routes: vec![
            RouteDescriptor {
                path: "/health".into(),
                methods: vec!["GET".into()],
            },
            RouteDescriptor {
                path: "/users".into(),
                methods: vec!["GET".into(), "POST".into()],
            },
        ],
    };

    let workers = WorkersStub {
        workers: vec![BackgroundWorkerDescriptor {
            name: "ProcessEmail".into(),
            queue: Some("default".into()),
        }],
    };

    let scheduler = SchedulerStub {
        jobs: vec![SchedulerJobDescriptor {
            name: "nightly_cleanup".into(),
            schedule: "0 0 * * *".into(),
            command: "cleanup".into(),
            run_on_start: false,
            shell: false,
            tags: vec!["maintenance".into()],
        }],
    };

    let tasks = TasksStub {
        tasks: vec![
            TaskDescriptor {
                name: "cleanup".into(),
                detail: Some("Cleanup stale data".into()),
            },
            TaskDescriptor {
                name: "send_welcome_email".into(),
                detail: Some("Send welcome email".into()),
            },
        ],
    };

    let builder = GraphBuilder::new("demo-app", &routes, &workers, &scheduler, &tasks);

    let graph = builder.build();

    assert_eq!(graph.nodes.len(), 7);

    let app_node = find_node(&graph, |node| {
        matches!(&node.kind, ComponentKind::Application { .. })
    });
    assert!(matches!(&app_node.kind, ComponentKind::Application { name } if name == "demo-app"));

    let route_nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| matches!(&node.kind, ComponentKind::HttpRoute { .. }))
        .collect();
    assert_eq!(route_nodes.len(), 2);

    for route_node in route_nodes {
        let edge = GraphEdge {
            from: app_node.id.clone(),
            to: route_node.id.clone(),
            kind: EdgeKind::Contains,
        };
        assert!(
            graph.edges.contains(&edge),
            "missing edge from app to route"
        );
    }

    let scheduler_node = find_node(
        &graph,
        |node| matches!(&node.kind, ComponentKind::SchedulerJob { name, .. } if name == "nightly_cleanup"),
    );
    let cleanup_task_node = find_node(
        &graph,
        |node| matches!(&node.kind, ComponentKind::Task { name, .. } if name == "cleanup"),
    );

    let trigger_edge = GraphEdge {
        from: scheduler_node.id.clone(),
        to: cleanup_task_node.id.clone(),
        kind: EdgeKind::Triggers,
    };
    assert!(graph.edges.contains(&trigger_edge));
}

#[test]
fn scheduler_job_without_matching_task_has_no_trigger_edge() {
    let routes = RoutesStub { routes: vec![] };
    let workers = WorkersStub { workers: vec![] };
    let scheduler = SchedulerStub {
        jobs: vec![SchedulerJobDescriptor {
            name: "shell_command".into(),
            schedule: "*/5 * * * *".into(),
            command: "echo hello".into(),
            run_on_start: true,
            shell: true,
            tags: vec![],
        }],
    };
    let tasks = TasksStub { tasks: vec![] };

    let graph = GraphBuilder::new("demo", &routes, &workers, &scheduler, &tasks).build();

    let scheduler_node = find_node(
        &graph,
        |node| matches!(&node.kind, ComponentKind::SchedulerJob { name, .. } if name == "shell_command"),
    );

    assert!(graph
        .edges
        .iter()
        .filter(|edge| edge.from == scheduler_node.id)
        .all(|edge| edge.kind != EdgeKind::Triggers));
}
