//! This module contains the core components and traits for building a web
//! server application.
#[cfg(feature = "with-db")]
use {sea_orm::DatabaseConnection, std::path::Path};

use std::{
    any::{Any, TypeId},
    net::SocketAddr,
    sync::Arc,
};

use async_trait::async_trait;
use axum::Router as AxumRouter;
use dashmap::DashMap;

use crate::{
    bgworker::{self, Queue},
    boot::{shutdown_signal, BootResult, ServeParams, StartMode},
    cache::{self},
    config::Config,
    controller::{
        middleware::{self, MiddlewareLayer},
        AppRoutes,
    },
    environment::Environment,
    mailer::EmailSender,
    storage::Storage,
    task::Tasks,
    Result,
};

/// Type-safe heterogeneous storage for arbitrary application data
#[derive(Default, Debug)]
pub struct SharedStore {
    // Use DashMap for concurrent access with fine-grained locking
    storage: DashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl SharedStore {
    /// Insert a value of type T into the shared store
    ///
    /// # Example
    /// ```
    /// # use loco_rs::app::SharedStore;
    /// let shared_store = SharedStore::default();
    ///
    /// #[derive(Debug)]
    /// struct TestService {
    ///     name: String,
    ///     value: i32,
    /// }
    ///
    /// let service = TestService {
    ///     name: "test".to_string(),
    ///     value: 100,
    /// };
    ///
    /// shared_store.insert(service);
    /// assert!(shared_store.contains::<TestService>());
    /// ```
    pub fn insert<T: 'static + Send + Sync>(&self, val: T) {
        self.storage.insert(TypeId::of::<T>(), Box::new(val));
    }

    /// Remove a value of type T from the shared store
    ///
    /// Returns `Some(T)` if the value was present and removed, `None` otherwise.
    ///
    /// # Example
    /// ```
    /// # use loco_rs::app::SharedStore;
    /// let shared_store = SharedStore::default();
    ///
    /// struct TestService {
    ///     name: String,
    ///     value: i32,
    /// }
    ///
    /// let service = TestService {
    ///     name: "test".to_string(),
    ///     value: 100,
    /// };
    ///
    /// shared_store.insert(service);
    /// assert!(shared_store.contains::<TestService>());
    ///
    /// // Remove and get the value
    /// let removed_service_opt = shared_store.remove::<TestService>();
    /// assert!(removed_service_opt.is_some(), "Service should be present");
    /// // Assert fields individually instead of comparing the whole struct
    /// if let Some(removed_service) = removed_service_opt {
    ///      assert_eq!(removed_service.name, "test");
    ///      assert_eq!(removed_service.value, 100);
    /// }
    /// // Ensure it's gone
    /// assert!(!shared_store.contains::<TestService>());
    ///
    /// // Trying to remove again returns None
    /// assert!(shared_store.remove::<TestService>().is_none());
    /// ```
    #[must_use]
    pub fn remove<T: 'static + Send + Sync>(&self) -> Option<T> {
        self.storage
            .remove(&TypeId::of::<T>())
            .map(|(_, v)| v) // Extract the Box<dyn Any>
            .and_then(|any| any.downcast::<T>().ok()) // Downcast to Box<T>
            .map(|boxed| *boxed) // Dereference the Box<T> to get T
    }

    /// Get a reference to a value of type T from the shared store.
    ///
    /// Returns `None` if the value doesn't exist.
    /// The reference is valid for as long as the returned `RefGuard` is held.
    /// If you need to clone the value, you can do so directly from the
    /// returned reference, or use the `get` method instead.
    ///
    /// # Example
    /// ```
    /// # use loco_rs::app::SharedStore;
    /// let shared_store = SharedStore::default();
    ///
    /// #[derive(Clone)]
    /// struct TestService {
    ///     name: String,
    ///     value: i32,
    /// }
    ///
    /// let service = TestService {
    ///     name: "test".to_string(),
    ///     value: 100,
    /// };
    ///
    /// shared_store.insert(service);
    ///
    /// // Get a reference to the service
    /// let service_ref = shared_store.get_ref::<TestService>().expect("Service not found");
    /// // Access fields directly
    /// assert_eq!(service_ref.name, "test");
    /// assert_eq!(service_ref.value, 100);
    ///
    /// // Clone if needed (the field itself)
    /// let name_clone = service_ref.name.clone();
    /// assert_eq!(name_clone, "test");
    ///
    /// // Compute values from the reference
    /// let name_len = service_ref.name.len();
    /// assert_eq!(name_len, 4);
    /// ```
    #[must_use]
    pub fn get_ref<T: 'static + Send + Sync>(&self) -> Option<RefGuard<'_, T>> {
        let type_id = TypeId::of::<T>();
        self.storage.get(&type_id).map(|r| RefGuard::<T> {
            inner: r,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Get a clone of a value of type T from the shared store.
    /// Requires T to implement Clone.
    ///
    /// Returns `None` if the value doesn't exist.
    /// This method clones the stored value.
    /// If cloning is not desired or T does not implement Clone,
    /// use `get_ref` instead.
    ///
    /// # Example
    /// ```
    /// # use loco_rs::app::SharedStore;
    /// let shared_store = SharedStore::default();
    ///
    /// #[derive(Clone)]
    /// struct TestService {
    ///     name: String,
    ///     value: i32,
    /// }
    ///
    /// let service = TestService {
    ///     name: "test".to_string(),
    ///     value: 100,
    /// };
    ///
    /// shared_store.insert(service);
    ///
    /// // Get a clone of the service
    /// let service_clone_opt = shared_store.get::<TestService>();
    /// assert!(service_clone_opt.is_some(), "Service not found");
    /// // Assert fields individually
    /// if let Some(ref service_clone) = service_clone_opt {
    ///     assert_eq!(service_clone.name, "test");
    ///     assert_eq!(service_clone.value, 100);
    /// }
    /// ```
    #[must_use]
    pub fn get<T: 'static + Send + Sync + Clone>(&self) -> Option<T> {
        self.get_ref::<T>().map(|guard| (*guard).clone())
    }

    /// Check if the shared store contains a value of type T
    ///
    /// # Example
    /// ```
    /// # use loco_rs::app::SharedStore;
    /// let shared_store = SharedStore::default();
    ///
    /// struct TestService {
    ///     name: String,
    ///     value: i32,
    /// }
    ///
    /// let service = TestService {
    ///     name: "test".to_string(),
    ///     value: 100,
    /// };
    ///
    /// shared_store.insert(service);
    /// assert!(shared_store.contains::<TestService>());
    /// assert!(!shared_store.contains::<String>());
    /// ```
    #[must_use]
    pub fn contains<T: 'static + Send + Sync>(&self) -> bool {
        self.storage.contains_key(&TypeId::of::<T>())
    }
}

// A wrapper around DashMap's Ref type that erases the exact type
// but provides deref to the target type
pub struct RefGuard<'a, T: 'static + Send + Sync> {
    inner: dashmap::mapref::one::Ref<'a, TypeId, Box<dyn Any + Send + Sync>>,
    _phantom: std::marker::PhantomData<&'a T>,
}

impl<T: 'static + Send + Sync> std::ops::Deref for RefGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // This is safe because we only create a RefGuard for a specific type
        // after looking it up by its TypeId
        self.inner
            .value()
            .downcast_ref::<T>()
            .expect("Type mismatch in RefGuard")
    }
}

/// Represents the application context for a web server.
///
/// This struct encapsulates various components and configurations required by
/// the web server to operate. It is typically used to store and manage shared
/// resources and settings that are accessible throughout the application's
/// lifetime.
#[derive(Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct AppContext {
    /// The environment in which the application is running.
    pub environment: Environment,
    #[cfg(feature = "with-db")]
    /// A database connection used by the application.
    pub db: DatabaseConnection,
    /// Queue provider
    pub queue_provider: Option<Arc<bgworker::Queue>>,
    /// Configuration settings for the application
    pub config: Config,
    /// An optional email sender component that can be used to send email.
    pub mailer: Option<EmailSender>,
    // An optional storage instance for the application
    pub storage: Arc<Storage>,
    // Cache instance for the application
    pub cache: Arc<cache::Cache>,
    /// Shared store for arbitrary application data
    pub shared_store: Arc<SharedStore>,
}

/// A trait that defines hooks for customizing and extending the behavior of a
/// web server application.
///
/// Users of the web server application should implement this trait to customize
/// the application's routing, worker connections, task registration, and
/// database actions according to their specific requirements and use cases.
#[async_trait]
pub trait Hooks: Send {
    /// Defines the composite app version
    #[must_use]
    fn app_version() -> String {
        "dev".to_string()
    }
    /// Defines the crate name
    ///
    /// Example
    /// ```rust
    /// fn app_name() -> &'static str {
    ///     env!("CARGO_CRATE_NAME")
    /// }
    /// ```
    fn app_name() -> &'static str;

    /// Initializes and boots the application based on the specified mode and
    /// environment.
    ///
    /// The boot initialization process may vary depending on whether a DB
    /// migrator is used or not.
    ///
    /// # Examples
    ///
    /// With DB:
    /// ```rust,ignore
    /// async fn boot(mode: StartMode, environment: &str, config: Config) -> Result<BootResult> {
    ///     create_app::<Self, Migrator>(mode, environment, config).await
    /// }
    /// ````
    ///
    /// Without DB:
    /// ```rust,ignore
    /// async fn boot(mode: StartMode, environment: &str, config: Config) -> Result<BootResult> {
    ///     create_app::<Self>(mode, environment, config).await
    /// }
    /// ````
    ///
    ///
    /// # Errors
    /// Could not boot the application
    async fn boot(mode: StartMode, environment: &Environment, config: Config)
        -> Result<BootResult>;

    /// Start serving the Axum web application on the specified address and
    /// port.
    ///
    /// # Returns
    /// A Result indicating success () or an error if the server fails to start.
    async fn serve(app: AxumRouter, ctx: &AppContext, serve_params: &ServeParams) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(&format!(
            "{}:{}",
            serve_params.binding, serve_params.port
        ))
        .await?;

        let cloned_ctx = ctx.clone();
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            tracing::info!("shutting down...");
            Self::on_shutdown(&cloned_ctx).await;
        })
        .await?;

        Ok(())
    }

    /// Override and return `Ok(true)` to provide an alternative logging and
    /// tracing stack of your own.
    /// When returning `Ok(true)`, Loco will *not* initialize its own logger,
    /// so you should set up a complete tracing and logging stack.
    ///
    /// # Errors
    /// If fails returns an error
    fn init_logger(_ctx: &AppContext) -> Result<bool> {
        Ok(false)
    }

    /// Loads the configuration settings for the application based on the given environment.
    ///
    /// This function is responsible for retrieving the configuration for the application
    /// based on the current environment.
    async fn load_config(env: &Environment) -> Result<Config> {
        env.load()
    }

    /// Returns the initial Axum router for the application, allowing the user
    /// to control the construction of the Axum router. This is where a fallback
    /// handler can be installed before middleware or other routes are added.
    ///
    /// # Errors
    /// Return an [`Result`] when the router could not be created
    async fn before_routes(_ctx: &AppContext) -> Result<AxumRouter<AppContext>> {
        Ok(AxumRouter::new())
    }

    /// Invoke this function after the Loco routers have been constructed. This
    /// function enables you to configure custom Axum logics, such as layers,
    /// that are compatible with Axum.
    ///
    /// # Errors
    /// Axum router error
    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router)
    }

    /// Provide a list of initializers
    /// An initializer can be used to seamlessly add functionality to your app
    /// or to initialize some aspects of it.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Provide a list of middlewares
    #[must_use]
    fn middlewares(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>> {
        middleware::default_middleware_stack(ctx)
    }

    /// Calling the function before run the app
    /// You can now code some custom loading of resources or other things before
    /// the app runs
    async fn before_run(_app_context: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Defines the application's routing configuration.
    fn routes(_ctx: &AppContext) -> AppRoutes;

    // Provides the options to change Loco [`AppContext`] after initialization.
    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        Ok(ctx)
    }

    /// Connects custom workers to the application using the provided
    /// [`Processor`] and [`AppContext`].
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()>;

    /// Registers custom tasks with the provided [`Tasks`] object.
    fn register_tasks(tasks: &mut Tasks);

    /// Truncates the database as required. Users should implement this
    /// function. The truncate controlled from the [`crate::config::Database`]
    /// by changing dangerously_truncate to true (default false).
    /// Truncate can be useful when you want to truncate the database before any
    /// test.
    #[cfg(feature = "with-db")]
    async fn truncate(_ctx: &AppContext) -> Result<()>;

    /// Seeds the database with initial data.
    #[cfg(feature = "with-db")]
    async fn seed(_ctx: &AppContext, path: &Path) -> Result<()>;

    /// Called when the application is shutting down.
    /// This function allows users to perform any necessary cleanup or final
    /// actions before the application stops completely.
    async fn on_shutdown(_ctx: &AppContext) {}
}

/// An initializer.
/// Initializers should be kept in `src/initializers/`
#[async_trait]
// <snip id="initializers-trait">
pub trait Initializer: Sync + Send {
    /// The initializer name or identifier
    fn name(&self) -> String;

    /// Occurs after the app's `before_run`.
    /// Use this to for one-time initializations, load caches, perform web
    /// hooks, etc.
    async fn before_run(&self, _app_context: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Occurs after the app's `after_routes`.
    /// Use this to compose additional functionality and wire it into an Axum
    /// Router
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router)
    }
}
// </snip>

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_cfg::app::get_app_context;

    struct TestService {
        name: String,
        value: i32,
    }

    #[derive(Clone)]
    struct CloneableTestService {
        name: String,
        value: i32,
    }

    #[test]
    fn test_extensions_insert_and_get() {
        // Setup
        let shared_store = SharedStore::default();

        shared_store.insert(42i32);
        assert_eq!(shared_store.get::<i32>().expect("Value should exist"), 42);

        let service = TestService {
            name: "test".to_string(),
            value: 100,
        };

        shared_store.insert(service);

        let service_ref_opt = shared_store.get_ref::<TestService>();
        assert!(service_ref_opt.is_some(), "Service ref should exist");
        if let Some(service_ref) = service_ref_opt {
            assert_eq!(service_ref.name, "test");
            assert_eq!(service_ref.value, 100);
            let name_clone = service_ref.name.clone();
            assert_eq!(name_clone, "test");
        } else {
            panic!("Should have gotten Some(service_ref)");
        }
    }

    #[test]
    fn test_extensions_get_without_clone() {
        let shared_store = SharedStore::default();

        let service = TestService {
            name: "test_direct".to_string(),
            value: 100,
        };
        shared_store.insert(service);

        let service_ref_opt = shared_store.get_ref::<TestService>();
        assert!(service_ref_opt.is_some(), "Service ref should exist");
        if let Some(service_ref) = service_ref_opt {
            assert_eq!(service_ref.name, "test_direct");
            assert_eq!(service_ref.value, 100);
        } else {
            panic!("Should have gotten Some(service_ref)");
        }

        let name_len_opt = shared_store.get_ref::<TestService>().map(|r| r.name.len());
        assert!(
            name_len_opt.is_some(),
            "Service ref should exist for len check"
        );
        assert_eq!(name_len_opt.unwrap(), 11);

        let value_opt = shared_store.get_ref::<TestService>().map(|r| r.value);
        assert!(
            value_opt.is_some(),
            "Service ref should exist for value check"
        );
        assert_eq!(value_opt.unwrap(), 100);
    }

    #[test]
    fn test_extensions_remove() {
        let shared_store = SharedStore::default();

        shared_store.insert(42i32);
        assert!(shared_store.contains::<i32>());
        assert_eq!(shared_store.remove::<i32>(), Some(42));
        assert!(!shared_store.contains::<i32>());
        assert_eq!(shared_store.remove::<i32>(), None);

        let service = TestService {
            name: "rem".to_string(),
            value: 50,
        };
        shared_store.insert(service);
        assert!(shared_store.contains::<TestService>());
        let removed_opt = shared_store.remove::<TestService>();
        assert!(removed_opt.is_some());
        if let Some(removed) = removed_opt {
            assert_eq!(removed.name, "rem");
            assert_eq!(removed.value, 50);
        } else {
            panic!("Removed option should be Some");
        }
        assert!(!shared_store.contains::<TestService>());
        assert!(shared_store.remove::<TestService>().is_none());
    }

    #[test]
    fn test_extensions_contains() {
        let shared_store = SharedStore::default();

        shared_store.insert(42i32);
        shared_store.insert(TestService {
            name: "contains".to_string(),
            value: 1,
        });

        assert!(shared_store.contains::<i32>());
        assert!(shared_store.contains::<TestService>());
        assert!(!shared_store.contains::<String>());
        assert!(!shared_store.contains::<CloneableTestService>());
    }

    #[test]
    fn test_extensions_get_cloned() {
        let shared_store = SharedStore::default();

        shared_store.insert(42i32);
        assert_eq!(shared_store.get::<i32>(), Some(42));
        assert!(shared_store.contains::<i32>());

        let service = CloneableTestService {
            name: "cloned_test".to_string(),
            value: 200,
        };
        shared_store.insert(service.clone());

        let service_clone_opt = shared_store.get::<CloneableTestService>();
        assert!(service_clone_opt.is_some(), "Cloned service should exist");
        if let Some(ref service_clone) = service_clone_opt {
            assert_eq!(service_clone.name, "cloned_test");
            assert_eq!(service_clone.value, 200);
        } else {
            panic!("Should have gotten Some(service_clone)");
        }

        assert!(shared_store.contains::<CloneableTestService>());
        let original_ref_opt = shared_store.get_ref::<CloneableTestService>();
        assert!(original_ref_opt.is_some(), "Original ref should exist");
        if let Some(original_ref) = original_ref_opt {
            assert_eq!(original_ref.name, "cloned_test");
            assert_eq!(original_ref.value, 200);
        } else {
            panic!("Should have gotten Some(original_ref)");
        }

        assert_eq!(shared_store.get::<String>(), None);
        assert!(shared_store.get::<CloneableTestService>().is_some());
        // The following line correctly fails to compile because TestService doesn't impl Clone,
        // which is required by the `get` method.
        // let non_existent_clone = shared_store.get::<TestService>();
    }

    #[tokio::test]
    async fn test_app_context_extensions() {
        let ctx = get_app_context().await;

        let service_cloneable = CloneableTestService {
            name: "app_context_test_cloneable".to_string(),
            value: 42,
        };
        ctx.shared_store.insert(service_cloneable.clone());

        let ref_opt = ctx.shared_store.get_ref::<CloneableTestService>();
        assert!(ref_opt.is_some(), "Cloneable service ref should exist");
        if let Some(service_ref) = ref_opt {
            assert_eq!(service_ref.name, "app_context_test_cloneable");
            assert_eq!(service_ref.value, 42);
        } else {
            panic!("Should have gotten Some(service_ref)");
        }

        let clone_opt = ctx.shared_store.get::<CloneableTestService>();
        assert!(clone_opt.is_some(), "Should get cloned service");
        if let Some(service_clone) = clone_opt {
            assert_eq!(service_clone.name, "app_context_test_cloneable");
            assert_eq!(service_clone.value, 42);
        } else {
            panic!("Should have gotten Some(service_clone)");
        }

        assert!(ctx.shared_store.contains::<CloneableTestService>());
        assert!(!ctx.shared_store.contains::<String>());

        let removed_cloneable_opt = ctx.shared_store.remove::<CloneableTestService>();
        assert!(removed_cloneable_opt.is_some());
        if let Some(removed) = removed_cloneable_opt {
            assert_eq!(removed.name, "app_context_test_cloneable");
            assert_eq!(removed.value, 42);
        } else {
            panic!("Removed cloneable option should be Some");
        }
        assert!(!ctx.shared_store.contains::<CloneableTestService>());

        let service_non_cloneable = TestService {
            name: "app_context_test_non_cloneable".to_string(),
            value: 99,
        };
        ctx.shared_store.insert(service_non_cloneable);

        let non_clone_ref_opt = ctx.shared_store.get_ref::<TestService>();
        assert!(
            non_clone_ref_opt.is_some(),
            "Non-cloneable service ref should exist"
        );
        if let Some(service_ref) = non_clone_ref_opt {
            assert_eq!(service_ref.name, "app_context_test_non_cloneable");
            assert_eq!(service_ref.value, 99);
        } else {
            panic!("Should have gotten Some(service_ref)");
        }

        assert!(ctx.shared_store.contains::<TestService>());

        let removed_non_cloneable_opt = ctx.shared_store.remove::<TestService>();
        assert!(removed_non_cloneable_opt.is_some());
        if let Some(removed) = removed_non_cloneable_opt {
            assert_eq!(removed.name, "app_context_test_non_cloneable");
            assert_eq!(removed.value, 99);
        } else {
            panic!("Removed non-cloneable option should be Some");
        }
        assert!(!ctx.shared_store.contains::<TestService>());
    }
}
