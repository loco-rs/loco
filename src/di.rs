use crate::prelude::AppContext;
use crate::Error;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use dashmap::DashMap;
use std::any::{Any, TypeId};

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
    pub fn remove<T: Any + Send + Sync + 'static>(&self, qualifier: Option<String>) -> Option<T> {
        self.services
            .remove(&(TypeId::of::<T>(), qualifier))
            .map(|f| f.1)
            .and_then(|any| any.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }
}

/// An extractor that streamlines the process of getting a service from the `DiContainer`.
pub struct Injectable<T>(pub T);

impl<T> FromRequestParts<AppContext> for Injectable<T>
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

        Ok(Injectable(instance))
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
}
