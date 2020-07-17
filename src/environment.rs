use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

pub type ServiceAny = dyn Any;
type ServiceMap = HashMap<&'static str, Box<dyn AsAny>>;

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

    pub fn get<T>(&self) -> &mut T
    where
        T: Service + 'static,
    {
        assert!(
            self.bootstrap_complete.load(Ordering::Relaxed),
            "tried to get {} during bootstrap",
            T::name()
        );

        // DNC need interior mutability here, we don't care if others modify the service?
        // does declaring ourselves Send + Sync in spite of that screw thing up?
        // how should we declare interior mutability
        let res: &mut T = Self::downcast::<T>(self.services.get_mut(T::name()).expect("service exists"));
        return res;
    }

    pub fn get_mut<T>(&self) -> RefMut<T>
    where
        T: Service + 'static,
    {
        assert!(
            self.bootstrap_complete.load(Ordering::Relaxed),
            "tried to get {} during bootstrap",
            T::name()
        );

        if let Ok(services) = self.services.try_borrow_mut() {
            return RefMut::map(services, |t| {
                Self::downcast_mut::<T>(t.get_mut(T::name()).expect("service exists"))
            });
        } else {
            panic!("could not borrow {}, it is already borrowed!", T::name());
        }
    }

    fn downcast<T>(s: &Box<dyn AsAny>) -> &T
    where
        T: Service + 'static,
    {
        s.as_any().downcast_ref::<T>().expect("right downcast")
    }

    fn downcast_mut<T>(s: &mut Box<dyn AsAny>) -> &mut T
    where
        T: Service + 'static,
    {
        s.as_any_mut().downcast_mut::<T>().expect("right downcast")
    }
}

pub struct ServiceKit {
    env: Weak<Environment>,
    deps: Vec<&'static str>,
}

impl ServiceKit {
    fn with_env<'a>(env_owned: Arc<Environment>, env: &'a mut Environment) -> ServiceKitProto<'a> {
        ServiceKitProto {
            env_owned,
            env,
            deps: Vec::<&'static str>::new(),
        }
    }
}

unsafe impl Send for ServiceKit {}
unsafe impl Sync for ServiceKit {}

struct ServiceKitProto<'a> {
    env_owned: Arc<Environment>,
    env: &'a mut Environment,
    deps: Vec<&'static str>,
}

impl<'a> ServiceKitProto<'a> {
    fn with_dep<T: Service + 'static>(mut self) -> ServiceKitProto<'a> {
        self.env.register::<T>(Arc::clone(&self.env_owned));
        self.deps.push(T::name());
        self
    }

    fn new(self) -> ServiceKit {
        // Bootstrap all services requested in here? Or should the environment do it?
        ServiceKit {
            env: Arc::downgrade(&self.env_owned),
            deps: self.deps,
        }
    }
}

pub trait Service: AsAny + Daemon + Start + Send + Sync {}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait Daemon {
    fn name() -> &'static str;
}

pub trait Start {
    fn start(env_owned: Arc<Environment>, env: &mut Environment) -> Self;
}

// TODO I assume this won't work for generic types, but a decent first pass
macro_rules! make_service {
    ( $struct_name:ident ) => {
        impl Service for $struct_name {}

        impl Daemon for $struct_name {
            fn name() -> &'static str {
                stringify!($struct_name)
            }
        }

        impl AsAny for $struct_name {
            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
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

    impl Start for Dependency {
        fn start(env_owned: Arc<Environment>, env: &mut Environment) -> Dependency {
            Dependency {}
        }
    }

    struct TestService {
        kit: ServiceKit,
        field: i32,
    }

    impl Start for TestService {
        fn start(env_owned: Arc<Environment>, env: &mut Environment) -> TestService {
            TestService {
                kit: ServiceKit::with_env(env_owned, env)
                    .with_dep::<Dependency>()
                    .new(),
                field: 0,
            }
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
    fn allows_mutable_borrow_after_const_borrow() {
        let e = create_environment();

        {
            let _b = e.get::<TestService>();
        }

        {
            let mut _b = e.get_mut::<TestService>();
        }
    }

    #[test]
    fn allows_const_borrow_after_mutable_borrow() {
        let e = create_environment();

        {
            let mut _b = e.get_mut::<TestService>();
        }

        {
            let _b = e.get::<TestService>();
        }
    }

    #[test]
    fn implements_send_and_sync() {
        assert_impl_all!(Environment: Send, Sync);
    }

    #[test]
    #[should_panic]
    fn panics_on_second_mutable_borrow() {
        let e = create_environment();

        let mut _b0 = e.get_mut::<TestService>();
        let mut _b1 = e.get_mut::<TestService>();
    }

    #[test]
    #[should_panic]
    fn panics_on_const_borrow_after_mutable_borrow() {
        let e = create_environment();

        let mut _b0 = e.get_mut::<TestService>();
        let _b1 = e.get::<TestService>();
    }

    #[test]
    #[should_panic]
    fn panics_on_mutable_borrow_after_const_borrow() {
        let e = create_environment();

        let _b0 = e.get::<TestService>();
        let mut _b1 = e.get_mut::<TestService>();
    }

    fn create_environment() -> Arc<Environment> {
        let (owned, env) = Environment::bootstrap();
        let env = unsafe { &mut *env };
        env.register::<TestService>(Arc::clone(&owned));
        env.finish_bootstrap();
        return owned;
    }
}
