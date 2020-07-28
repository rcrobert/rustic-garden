pub use std::any::Any;
use std::collections::HashMap;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

type ServiceAnonymous = dyn AsAny + Send + Sync;
type ServiceMap = HashMap<&'static str, Box<ServiceAnonymous>>;

/// An environment containing various services.
pub struct Environment {
    services: ServiceMap,
    bootstrap_complete: AtomicBool,
}

impl Environment {
    /// Create a new, empty environment.
    pub fn new() -> Environment {
        Environment {
            services: HashMap::new(),
            bootstrap_complete: AtomicBool::new(false),
        }
    }

    pub fn bootstrap() -> (Arc<Environment>, *mut Environment) {
        let env_owned = Arc::new(Environment::new());
        let env_ptr: *mut Environment = Arc::into_raw(env_owned) as *mut Environment;
        assert!(env_ptr != ptr::null_mut());
        let env_owned = unsafe { Arc::from_raw(env_ptr) };
        return (env_owned, env_ptr);
    }

    pub fn finish_bootstrap(&mut self) {
        let already_finished =
            self.bootstrap_complete
                .compare_and_swap(false, true, Ordering::Relaxed);
        if already_finished {
            panic!("tried to finish bootstrap multiple times");
        }
    }

    /// Registers a new service with this environment.
    pub fn register<T>(&mut self, env_owned: Arc<Environment>)
    where
        T: Service + 'static,
    {
        assert!(
            !self.bootstrap_complete.load(Ordering::Relaxed),
            "tried to register {} after bootstrap completed",
            T::name()
        );

        let new_service = T::start(env_owned, self);

        self.services.insert(T::name(), Box::new(new_service));
    }

    pub fn get<T>(&self) -> &T
    where
        T: Service + 'static,
    {
        assert!(
            self.bootstrap_complete.load(Ordering::Relaxed),
            "tried to get {} during bootstrap",
            T::name()
        );

        let res: &T = Self::downcast::<T>(self.services.get(T::name()).expect("service exists"));
        return res;
    }

    fn downcast<T>(s: &Box<ServiceAnonymous>) -> &T
    where
        T: Service + 'static,
    {
        s.as_any().downcast_ref::<T>().expect("right downcast")
    }
}

pub struct ServiceKit {
    env: Weak<Environment>,
    deps: Vec<&'static str>,
}

impl ServiceKit {
    pub fn with_env<'a>(env_owned: Arc<Environment>, env: &'a mut Environment) -> ServiceKitProto<'a> {
        ServiceKitProto {
            env_owned,
            env,
            deps: Vec::<&'static str>::new(),
        }
    }
}

unsafe impl Send for ServiceKit {}
unsafe impl Sync for ServiceKit {}

pub struct ServiceKitProto<'a> {
    env_owned: Arc<Environment>,
    env: &'a mut Environment,
    deps: Vec<&'static str>,
}

impl<'a> ServiceKitProto<'a> {
    pub fn with_dep<T: Service + 'static>(mut self) -> ServiceKitProto<'a> {
        self.env.register::<T>(Arc::clone(&self.env_owned));
        self.deps.push(T::name());
        self
    }

    pub fn new(self) -> ServiceKit {
        // Bootstrap all services requested in here? Or should the environment do it?
        ServiceKit {
            env: Arc::downgrade(&self.env_owned),
            deps: self.deps,
        }
    }
}

pub trait Service: AsAny + Send + Sync {
    fn name() -> &'static str;
    fn start(env_owned: Arc<Environment>, env: &mut Environment) -> Self;
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

// TODO I assume this won't work for generic types, but a decent first pass
macro_rules! make_service {
    ( $struct_name:ident ) => {
        impl AsAny for $struct_name {
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dependency {}
    make_service!(Dependency);

    impl Service for Dependency {
        fn start(env_owned: Arc<Environment>, env: &mut Environment) -> Dependency {
            Dependency {}
        }

        fn name() -> &'static str {
            "Dependency"
        }
    }

    struct TestService {
        kit: ServiceKit,
        field: i32,
    }

    impl Service for TestService {
        fn start(env_owned: Arc<Environment>, env: &mut Environment) -> TestService {
            TestService {
                kit: ServiceKit::with_env(env_owned, env)
                    .with_dep::<Dependency>()
                    .new(),
                field: 0,
            }
        }

        fn name() -> &'static str {
            "TestService"
        }
    }

    make_service!(TestService);

    /// Test the make_service! macro preserves fields
    #[test]
    fn still_has_fields() {
        let e = create_environment();

        e.get::<TestService>().field;
    }

    #[test]
    fn allows_multiple_const_borrows() {
        let e = create_environment();

        let _b0 = e.get::<TestService>();
        let _b1 = e.get::<TestService>();
    }

    #[test]
    fn implements_send_and_sync() {
        assert_impl_all!(Environment: Send, Sync);
    }

    fn create_environment() -> Arc<Environment> {
        let (owned, env) = Environment::bootstrap();
        let env = unsafe { &mut *env };
        env.register::<TestService>(Arc::clone(&owned));
        env.finish_bootstrap();
        return owned;
    }
}
