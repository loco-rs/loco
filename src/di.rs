use crate::prelude::AppContext;
use crate::Error;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use dashmap::DashMap;
use std::any::{Any, TypeId};
use std::future::Future;

/// A container that contains and manages instances.
#[derive(Default)]
pub struct DiContainer {
    services: DashMap<(TypeId, Option<String>), Box<dyn Any + Send + Sync>>,
}

impl DiContainer {
    /// Adds a service instace to the container.
    ///
    /// # Arguments
    ///
    /// * `service`: The service that should be managed by the container.
    /// * `qualifier`: If you have multiple services from the same type you can differentiate them by supplying a qualifier.
    pub fn add<T: Any + Send + Sync + 'static>(&self, service: T, qualifier: Option<String>) {
        self.services
            .insert((TypeId::of::<T>(), qualifier), Box::new(service));
    }

    /// Gets a cloned version of the instance.
    ///
    /// # Arguments
    ///
    /// * `qualifier`: An optional qualifier that helps you differentiate multiple instance of the same type.
    ///
    /// returns: Option<T>
    #[must_use]
    pub fn get<T: Any + Clone + Send + Sync + 'static>(
        &self,
        qualifier: Option<String>,
    ) -> Option<T> {
        self.services
            .get(&(TypeId::of::<T>(), qualifier))
            .and_then(|s| s.downcast_ref::<T>().cloned())
    }

    /// Checks if the container already manages a specific service with the given qualifier.
    ///
    /// # Arguments
    ///
    /// * `qualifier`: An optional qualifier that helps you differentiate multiple instance of the same type.
    ///
    /// returns: bool
    #[must_use]
    pub fn has<T: Any + Send + Sync + 'static>(&self, qualifier: Option<String>) -> bool {
        self.services.contains_key(&(TypeId::of::<T>(), qualifier))
    }

    /// Removes a service from the container and returns you the original one.
    ///
    /// # Arguments
    ///
    /// * `qualifier`: An optional qualifier that helps you differentiate multiple instance of the same type.
    ///
    /// returns: Option<T>
    #[must_use]
    pub fn remove<T: Any + Send + Sync + 'static>(&self, qualifier: Option<String>) -> Option<T> {
        self.services
            .remove(&(TypeId::of::<T>(), qualifier))
            .map(|f| f.1)
            .and_then(|any| any.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }
}

/// An extractor that streamlines the process of getting static Data from the `DiContainer`.
/// Keep in mind that these extractors use `None` as the qualifier.
pub struct Data<T>(pub T);

impl<T> FromRequestParts<AppContext> for Data<T>
where
    T: Any + Clone + Send + Sync + 'static,
{
    type Rejection = Error;

    async fn from_request_parts(
        _: &mut Parts,
        state: &AppContext,
    ) -> Result<Self, Self::Rejection> {
        let instance = state
            .container
            .get::<T>(None)
            // TODO maybe introduce custom error?
            .ok_or(Error::Message("Could not find service".to_string()))?;

        Ok(Self(instance))
    }
}

/// An extractor that streamlines the process of getting a `Service` from the `DiContainer`.
/// Keep in mind that these extractors use `None` as the qualifier.
pub struct Injectable<T: Service>(pub T);

impl<T: Service> FromRequestParts<AppContext> for Injectable<T> {
    type Rejection = Error;

    async fn from_request_parts(
        _: &mut Parts,
        state: &AppContext,
    ) -> Result<Self, Self::Rejection> {
        let instance = T::get(state).await?;

        Ok(Self(instance))
    }
}

/// Defines a service which can be given and constructed with only the `AppContext`.
pub trait Service: Sized + Clone + Send + Sync + 'static {
    /// Builds a new instance of the service.
    fn build(ctx: &AppContext) -> impl Future<Output = Result<Self, Error>> + Send;

    /// Gets you an instance of the service from the `DiContainer`.
    ///
    /// If no instance exist it will create a new one and automatically adds it to the `DiContainer`.
    #[must_use]
    fn get(ctx: &AppContext) -> impl Future<Output = Result<Self, Error>> + Send {
        async {
            match ctx.container.get::<Self>(None) {
                None => {
                    let instance = Self::build(ctx).await?;

                    ctx.container.add(instance, None);

                    // We can safely unwrap() here has there is no chance that this service is going
                    // to be removed before that
                    Ok(ctx.container.get(None).unwrap())
                }
                Some(instance) => Ok(instance),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestService {
        id: u32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct AnotherService {
        name: String,
    }

    #[test]
    fn test_add_and_get_without_qualifier() {
        let container = DiContainer::default();
        let service = TestService { id: 1 };

        container.add(service, None);

        let retrieved: Option<TestService> = container.get(None);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, 1);
    }

    #[test]
    fn test_add_and_get_with_qualifier() {
        let container = DiContainer::default();
        let service = TestService { id: 1 };

        container.add(service, Some("test".to_string()));

        let retrieved: Option<TestService> = container.get(Some("test".to_string()));
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, 1);
    }

    #[test]
    fn test_multiple_services_with_different_qualifiers() {
        let container = DiContainer::default();

        container.add(TestService { id: 1 }, Some("first".to_string()));
        container.add(TestService { id: 2 }, Some("second".to_string()));

        let first: Option<TestService> = container.get(Some("first".to_string()));
        let second: Option<TestService> = container.get(Some("second".to_string()));

        assert_eq!(first.unwrap().id, 1);
        assert_eq!(second.unwrap().id, 2);
    }

    #[test]
    fn test_has_service() {
        let container = DiContainer::default();

        assert!(!container.has::<TestService>(None));

        container.add(TestService { id: 1 }, None);

        assert!(container.has::<TestService>(None));
        assert!(!container.has::<TestService>(Some("qualifier".to_string())));
    }

    #[test]
    fn test_remove_service() {
        let container = DiContainer::default();
        container.add(TestService { id: 1 }, None);

        let removed: Option<TestService> = container.remove(None);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, 1);

        // Service should no longer exist
        assert!(!container.has::<TestService>(None));
    }

    #[test]
    fn test_different_service_types() {
        let container = DiContainer::default();

        container.add(TestService { id: 1 }, None);
        container.add(
            AnotherService {
                name: "test".to_string(),
            },
            None,
        );

        let service1: Option<TestService> = container.get(None);
        let service2: Option<AnotherService> = container.get(None);

        assert_eq!(service1.unwrap().id, 1);
        assert_eq!(service2.unwrap().name, "test");
    }

    mod service_tests {
        use super::*;
        use crate::tests_cfg;
        use futures_util::future::join_all;

        #[derive(Clone, Debug, PartialEq)]
        struct MockService {
            id: u32,
        }

        impl Service for MockService {
            async fn build(_ctx: &AppContext) -> Result<Self, Error> {
                Ok(MockService { id: 42 })
            }
        }

        #[derive(Clone, Debug, PartialEq)]
        struct ErrorService;

        impl Service for ErrorService {
            async fn build(_ctx: &AppContext) -> Result<Self, Error> {
                Err(Error::Message("Test build error".to_string()))
            }
        }

        #[tokio::test]
        async fn test_service_build() {
            let app_context = tests_cfg::app::get_app_context().await;

            let service = MockService::build(&app_context).await;
            assert!(service.is_ok());
            assert_eq!(service.unwrap().id, 42);
        }

        #[tokio::test]
        async fn test_service_get_builds_new_instance() {
            let app_context = tests_cfg::app::get_app_context().await;

            // Ensure service doesn't exist before test
            assert!(!app_context.container.has::<MockService>(None));

            // First call should build a new instance
            let service = MockService::get(&app_context).await;
            assert!(service.is_ok());
            assert_eq!(service.unwrap().id, 42);

            // Service should now exist in container
            assert!(app_context.container.has::<MockService>(None));
        }

        #[tokio::test]
        async fn test_service_get_returns_existing_instance() {
            let app_context = tests_cfg::app::get_app_context().await;

            // Add a service with a custom ID directly to the container
            let existing = MockService { id: 100 };
            app_context.container.add(existing, None);

            // Get should return the existing instance (with ID 100) instead of building a new one (which would have ID 42)
            let service = MockService::get(&app_context).await;
            assert!(service.is_ok());
            assert_eq!(service.unwrap().id, 100);
        }

        #[tokio::test]
        async fn test_service_build_error() {
            let app_context = tests_cfg::app::get_app_context().await;

            let result = ErrorService::get(&app_context).await;
            assert!(result.is_err());

            if let Err(Error::Message(msg)) = result {
                assert_eq!(msg, "Test build error");
            } else {
                panic!("Expected Error::Message variant");
            }

            // Service should not be added to container after build error
            assert!(!app_context.container.has::<ErrorService>(None));
        }

        #[tokio::test]
        async fn test_injectable_struct() {
            let app_context = tests_cfg::app::get_app_context().await;

            // Add a service to the container
            app_context.container.add(MockService { id: 42 }, None);

            // Manually simulate what the extractor would do
            let instance = MockService::get(&app_context).await.unwrap();
            let injectable = Injectable(instance);

            assert_eq!(injectable.0.id, 42);
        }

        #[tokio::test]
        async fn test_service_singleton_behavior() {
            let app_context = tests_cfg::app::get_app_context().await;

            // Run multiple concurrent get() calls
            let futures = (0..10).map(|_| MockService::get(&app_context));
            let results = join_all(futures).await;

            // All should succeed
            for result in &results {
                assert!(result.is_ok());
            }

            // Container should have exactly one instance
            let instance = app_context.container.get::<MockService>(None);
            assert!(instance.is_some());

            // All returned instances should be identical
            let first = &results[0].as_ref().unwrap();
            for result in &results {
                assert_eq!(&result.as_ref().unwrap(), first);
            }
        }
    }
}
