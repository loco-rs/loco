use crate::prelude::AppContext;
use crate::Error;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use dashmap::DashMap;
use std::any::{Any, TypeId};

/// A container that contains and manages instances.
#[derive(Default)]
pub struct DiContainer {
    services: DashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl DiContainer {
    /// Adds a service instance to the container.
    ///
    /// # Arguments
    ///
    /// * `service`: The service that should be managed by the container.
    pub fn add<T: Any + Send + Sync + 'static>(&self, service: T) {
        self.services.insert(TypeId::of::<T>(), Box::new(service));
    }

    /// Gets a cloned version of the instance.
    ///
    /// returns: Option<T>
    #[must_use]
    pub fn get<T: Any + Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|s| s.downcast_ref::<T>().cloned())
    }

    /// Removes a service from the container and returns you the original one.
    ///
    /// returns: Option<T>
    #[must_use]
    pub fn remove<T: Any + Send + Sync + 'static>(&self) -> Option<T> {
        self.services
            .remove(&TypeId::of::<T>())
            .map(|f| f.1)
            .and_then(|any| any.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }

    /// Checks if the container already manages a specific service with the given qualifier.
    ///
    /// returns: bool
    #[must_use]
    pub fn has<T: Any + Send + Sync + 'static>(&self) -> bool {
        self.services.contains_key(&TypeId::of::<T>())
    }
}

/// An extractor that streamlines the process of getting static Data from the `DiContainer`.
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
            .get::<T>()
            // TODO maybe introduce custom error?
            .ok_or(Error::Message("Could not find service".to_string()))?;

        Ok(Self(instance))
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
    fn test_add_and_get() {
        let container = DiContainer::default();
        let service = TestService { id: 1 };

        container.add(service);

        let retrieved: Option<TestService> = container.get();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, 1);
    }

    #[test]
    fn test_multiple_services_of_different_types() {
        let container = DiContainer::default();

        container.add(TestService { id: 1 });
        container.add(AnotherService {
            name: "test".to_string(),
        });

        let test_service: Option<TestService> = container.get();
        let another_service: Option<AnotherService> = container.get();

        assert_eq!(test_service.unwrap().id, 1);
        assert_eq!(another_service.unwrap().name, "test");
    }

    #[test]
    fn test_has_service() {
        let container = DiContainer::default();

        assert!(!container.has::<TestService>());

        container.add(TestService { id: 1 });

        assert!(container.has::<TestService>());
    }

    #[test]
    fn test_remove_service() {
        let container = DiContainer::default();
        container.add(TestService { id: 1 });

        let removed: Option<TestService> = container.remove();
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, 1);

        // Service should no longer exist
        assert!(!container.has::<TestService>());
    }

    #[test]
    fn test_service_replacement() {
        let container = DiContainer::default();

        container.add(TestService { id: 1 });

        // Add a new service of the same type
        container.add(TestService { id: 2 });

        // Should get the most recently added service
        let service: Option<TestService> = container.get();
        assert_eq!(service.unwrap().id, 2);
    }

    #[test]
    fn test_different_service_types() {
        let container = DiContainer::default();

        container.add(TestService { id: 1 });
        container.add(AnotherService {
            name: "test".to_string(),
        });

        let service1: Option<TestService> = container.get();
        let service2: Option<AnotherService> = container.get();

        assert_eq!(service1.unwrap().id, 1);
        assert_eq!(service2.unwrap().name, "test");
    }
}
