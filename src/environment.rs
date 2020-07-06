use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

pub type ServiceAny = dyn Any;
type ServiceMap = HashMap<&'static str, Box<dyn Service>>;

/// An environment containing various services.
pub struct Environment {
    services: RefCell<ServiceMap>,
}

impl Environment {
    /// Create a new, empty environment.
    pub fn new() -> Environment {
        Environment {
            services: RefCell::new(HashMap::new()),
        }
    }

    /// Registers a new service with this environment.
    pub fn register<T>(&mut self, new_service: T)
    where
        T: Service + Daemon + 'static,
    {
        if let Ok(mut services) = self.services.try_borrow_mut() {
            services.insert(T::name(), Box::new(new_service));
        } else {
            panic!(
                "could not register {}, services are already borrowed mut! try registering at upstart",
                T::name()
            );
        }
    }

    pub fn get<T>(&self) -> Ref<T>
    where
        T: Service + Daemon + 'static,
    {
        if let Ok(services) = self.services.try_borrow() {
            return Ref::map(services, |t| {
                Self::downcast::<T>(t.get(T::name()).expect("service exists"))
            });
        } else {
            panic!(
                "could not borrow {}, it is already borrowed mut!",
                T::name()
            );
        }
    }

    pub fn get_mut<T>(&self) -> RefMut<T>
    where
        T: Service + Daemon + 'static,
    {
        if let Ok(services) = self.services.try_borrow_mut() {
            return RefMut::map(services, |t| {
                Self::downcast_mut::<T>(t.get_mut(T::name()).expect("service exists"))
            });
        } else {
            panic!("could not borrow {}, it is already borrowed!", T::name());
        }
    }

    fn downcast<T>(s: &Box<dyn Service>) -> &T
    where
        T: Service + Daemon + 'static,
    {
        s.as_any().downcast_ref::<T>().expect("right downcast")
    }

    fn downcast_mut<T>(s: &mut Box<dyn Service>) -> &mut T
    where
        T: Service + Daemon + 'static,
    {
        s.as_any_mut().downcast_mut::<T>().expect("right downcast")
    }
}

pub trait Service {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait Daemon {
    fn name() -> &'static str;

    // TODO this should have a construction method, so non-empty construct is supported for
    // services, then update register to call bind() instead of expecting an instance already
    // fn bind(env: &Environment) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {}

    impl Service for TestService {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    impl Daemon for TestService {
        fn name() -> &'static str {
            "TestService"
        }
    }

    #[test]
    fn allows_multiple_const_borrows() {
        let mut e = Environment::new();
        e.register::<TestService>(TestService {});

        let _b0 = e.get::<TestService>();
        let _b1 = e.get::<TestService>();
    }

    #[test]
    fn allows_mutable_borrow_after_const_borrow() {
        let mut e = Environment::new();
        e.register::<TestService>(TestService {});

        {
            let _b = e.get::<TestService>();
        }

        {
            let mut _b = e.get_mut::<TestService>();
        }
    }

    #[test]
    fn allows_const_borrow_after_mutable_borrow() {
        let mut e = Environment::new();
        e.register::<TestService>(TestService {});

        {
            let mut _b = e.get_mut::<TestService>();
        }

        {
            let _b = e.get::<TestService>();
        }
    }

    #[test]
    #[should_panic]
    fn panics_on_second_mutable_borrow() {
        let mut e = Environment::new();
        e.register::<TestService>(TestService {});

        let mut _b0 = e.get_mut::<TestService>();
        let mut _b1 = e.get_mut::<TestService>();
    }

    #[test]
    #[should_panic]
    fn panics_on_const_borrow_after_mutable_borrow() {
        let mut e = Environment::new();
        e.register::<TestService>(TestService {});

        let mut _b0 = e.get_mut::<TestService>();
        let _b1 = e.get::<TestService>();
    }

    #[test]
    #[should_panic]
    fn panics_on_mutable_borrow_after_const_borrow() {
        let mut e = Environment::new();
        e.register::<TestService>(TestService {});

        let _b0 = e.get::<TestService>();
        let mut _b1 = e.get_mut::<TestService>();
    }
}
