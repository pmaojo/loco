use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use serde::Serialize;

use crate::{
    app::AppContext,
    bgworker::Queue,
    controller::{AppRoutes, ListRoutes},
    scheduler,
    task::Tasks,
};

use super::domain::{
    ApplicationGraph, BackgroundWorkerDescriptor, BackgroundWorkerRepository, GraphBuilder,
    RouteDescriptor, RoutesRepository, SchedulerJobDescriptor, SchedulerRepository, TaskDescriptor,
    TaskRepository,
};

/// Cached data used by adapters to instantiate [`ApplicationGraphService`].
#[derive(Debug, Clone)]
pub struct GraphIntrospectionSeed {
    pub app_name: String,
    pub routes: Vec<RouteDescriptor>,
}

impl GraphIntrospectionSeed {
    /// Creates a new seed instance from owned data.
    #[must_use]
    pub fn new(app_name: impl Into<String>, routes: Vec<RouteDescriptor>) -> Self {
        Self {
            app_name: app_name.into(),
            routes,
        }
    }

    /// Instantiates an [`ApplicationGraphService`] bound to this seed.
    #[must_use]
    pub fn into_service<'a>(&'a self, context: &'a AppContext) -> ApplicationGraphService<'a> {
        ApplicationGraphService::from_route_descriptors(
            &self.app_name,
            self.routes.clone(),
            context,
        )
    }
}

/// Application port exposing read access to the introspection graph.
pub trait GraphQueryService {
    /// Collects a snapshot describing routes, dependencies and health information.
    fn snapshot(&self) -> GraphSnapshot;
}

/// Serializable representation of the application graph exposed to adapters.
#[derive(Debug, Clone, Serialize)]
pub struct GraphSnapshot {
    pub routes: Vec<RouteDescriptor>,
    pub dependencies: GraphDependencies,
    pub health: GraphHealth,
}

/// Collection of framework dependencies registered in the application.
#[derive(Debug, Clone, Serialize)]
pub struct GraphDependencies {
    pub background_workers: Vec<BackgroundWorkerDescriptor>,
    pub scheduler_jobs: Vec<SchedulerJobDescriptor>,
    pub tasks: Vec<TaskDescriptor>,
}

/// Health status for the introspection graph.
#[derive(Debug, Clone, Serialize)]
pub struct GraphHealth {
    pub ok: bool,
}

/// Service that adapts framework-specific data sources to the graph domain.
pub struct ApplicationGraphService<'a> {
    app_name: &'a str,
    routes: Vec<RouteDescriptor>,
    queue_provider: Option<Arc<Queue>>,
    scheduler_config: Option<&'a scheduler::Config>,
    context: &'a AppContext,
    task_registry: Option<&'a Tasks>,
}

impl<'a> ApplicationGraphService<'a> {
    /// Builds the service from the [`AppRoutes`] definition and application context.
    pub fn new(app_name: &'a str, routes: &'a AppRoutes, context: &'a AppContext) -> Self {
        let collected_routes = routes.collect();
        let descriptors = Self::collect_route_descriptors(&collected_routes);
        Self::from_route_descriptors(app_name, descriptors, context)
    }

    /// Builds the service from already collected routes, allowing tests to stub the data.
    pub fn from_list_routes(
        app_name: &'a str,
        routes: Vec<ListRoutes>,
        context: &'a AppContext,
    ) -> Self {
        let descriptors = Self::collect_route_descriptors(&routes);
        Self::from_route_descriptors(app_name, descriptors, context)
    }

    /// Builds the service from route descriptors, bypassing Axum specific metadata.
    pub fn from_route_descriptors(
        app_name: &'a str,
        routes: Vec<RouteDescriptor>,
        context: &'a AppContext,
    ) -> Self {
        Self {
            app_name,
            routes,
            queue_provider: context.queue_provider.clone(),
            scheduler_config: context.config.scheduler.as_ref(),
            context,
            task_registry: None,
        }
    }

    /// Overrides the queue provider dependency.
    pub fn with_queue_provider(mut self, queue_provider: Option<Arc<Queue>>) -> Self {
        self.queue_provider = queue_provider;
        self
    }

    /// Overrides the scheduler configuration dependency.
    pub fn with_scheduler_config(
        mut self,
        scheduler_config: Option<&'a scheduler::Config>,
    ) -> Self {
        self.scheduler_config = scheduler_config;
        self
    }

    /// Overrides the task registry dependency, bypassing the shared store lookup.
    pub fn with_task_registry(mut self, registry: Option<&'a Tasks>) -> Self {
        self.task_registry = registry;
        self
    }

    /// Overrides the collected route descriptors.
    pub fn with_routes(mut self, routes: Vec<RouteDescriptor>) -> Self {
        self.routes = routes;
        self
    }

    /// Materialises the [`ApplicationGraph`] using the domain builder.
    pub fn build_graph(&self) -> ApplicationGraph {
        GraphBuilder::new(self.app_name, self, self, self, self).build()
    }

    /// Aggregates route descriptors from collected [`ListRoutes`] data.
    #[must_use]
    pub fn collect_route_descriptors(routes: &[ListRoutes]) -> Vec<RouteDescriptor> {
        let mut aggregated: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for route in routes {
            let entry = aggregated.entry(route.uri.clone()).or_default();
            for method in &route.actions {
                entry.insert(method.to_string());
            }
        }

        aggregated
            .into_iter()
            .map(|(path, methods)| RouteDescriptor {
                path,
                methods: methods.into_iter().collect(),
            })
            .collect()
    }
}

impl GraphQueryService for ApplicationGraphService<'_> {
    fn snapshot(&self) -> GraphSnapshot {
        GraphSnapshot {
            routes: RoutesRepository::routes(self),
            dependencies: GraphDependencies {
                background_workers: BackgroundWorkerRepository::workers(self),
                scheduler_jobs: SchedulerRepository::jobs(self),
                tasks: TaskRepository::tasks(self),
            },
            health: GraphHealth { ok: true },
        }
    }
}

impl RoutesRepository for ApplicationGraphService<'_> {
    fn routes(&self) -> Vec<RouteDescriptor> {
        self.routes.clone()
    }
}

impl BackgroundWorkerRepository for ApplicationGraphService<'_> {
    fn workers(&self) -> Vec<BackgroundWorkerDescriptor> {
        let mut names = BTreeSet::new();

        if let Some(queue) = &self.queue_provider {
            match queue.as_ref() {
                #[cfg(feature = "bg_redis")]
                Queue::Redis(_, registry, _, _) => {
                    names.extend(read_registry_names(Arc::clone(registry)));
                }
                #[cfg(feature = "bg_pg")]
                Queue::Postgres(_, registry, _, _) => {
                    names.extend(read_registry_names(Arc::clone(registry)));
                }
                #[cfg(feature = "bg_sqlt")]
                Queue::Sqlite(_, registry, _, _) => {
                    names.extend(read_registry_names(Arc::clone(registry)));
                }
                Queue::None => {}
            }
        }

        names
            .into_iter()
            .map(|name| BackgroundWorkerDescriptor { name, queue: None })
            .collect()
    }
}

impl SchedulerRepository for ApplicationGraphService<'_> {
    fn jobs(&self) -> Vec<SchedulerJobDescriptor> {
        let mut result = Vec::new();

        if let Some(config) = self.scheduler_config {
            let mut jobs: Vec<_> = config.jobs.iter().collect();
            jobs.sort_by(|(left, _), (right, _)| left.cmp(right));

            for (name, job) in jobs {
                let mut tags = job.tags.clone().unwrap_or_default();
                tags.sort();
                tags.dedup();
                result.push(SchedulerJobDescriptor {
                    name: name.clone(),
                    schedule: job.cron.clone(),
                    command: job.run.clone(),
                    run_on_start: job.run_on_start,
                    shell: job.shell,
                    tags,
                });
            }
        }

        result
    }
}

impl TaskRepository for ApplicationGraphService<'_> {
    fn tasks(&self) -> Vec<TaskDescriptor> {
        if let Some(registry) = self.task_registry {
            return collect_tasks(registry);
        }

        if let Some(registry) = self.context.shared_store.get_ref::<Tasks>() {
            return collect_tasks(&registry);
        }

        Vec::new()
    }
}

/// Collects task descriptors from a registry and keeps the output sorted for determinism.
fn collect_tasks(tasks: &Tasks) -> Vec<TaskDescriptor> {
    let mut descriptors: Vec<_> = tasks
        .list()
        .into_iter()
        .map(|info| TaskDescriptor {
            name: info.name,
            detail: Some(info.detail),
        })
        .collect();
    descriptors.sort_by(|left, right| left.name.cmp(&right.name));
    descriptors
}

#[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
fn read_registry_names<R>(registry: Arc<tokio::sync::Mutex<R>>) -> Vec<String>
where
    R: RegistryExtractor,
{
    block_on_registry(async move {
        let guard = registry.lock().await;
        guard.handler_names()
    })
    .unwrap_or_default()
}

#[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
trait RegistryExtractor {
    fn handler_names(&self) -> Vec<String>;
}

#[cfg(feature = "bg_redis")]
impl RegistryExtractor for crate::bgworker::redis::JobRegistry {
    fn handler_names(&self) -> Vec<String> {
        self.handlers().keys().cloned().collect()
    }
}

#[cfg(feature = "bg_pg")]
impl RegistryExtractor for crate::bgworker::pg::JobRegistry {
    fn handler_names(&self) -> Vec<String> {
        self.handlers().keys().cloned().collect()
    }
}

#[cfg(feature = "bg_sqlt")]
impl RegistryExtractor for crate::bgworker::sqlt::JobRegistry {
    fn handler_names(&self) -> Vec<String> {
        self.handlers().keys().cloned().collect()
    }
}

#[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]
fn block_on_registry<F, T>(future: F) -> Option<T>
where
    F: std::future::Future<Output = T>,
{
    use tokio::runtime::{Builder, Handle};

    match Handle::try_current() {
        Ok(handle) => Some(handle.block_on(future)),
        Err(_) => Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .map(|rt| rt.block_on(future)),
    }
}
