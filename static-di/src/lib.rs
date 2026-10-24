use std::any::{TypeId, type_name};

pub trait Service {}

pub trait ServiceCollection {
    fn get_service<S: 'static>(&self) -> &S;
}

// Auto-Generated
pub trait ResolvedServiceCollection {
    type S1: 'static;
    type S2: 'static;
    fn t1(&self) -> &Self::S1;
    fn t2(&self) -> &Self::S2;
}

impl<T> ServiceCollection for T
where
    T: ResolvedServiceCollection,
{
    fn get_service<S: 'static>(&self) -> &S {
        // We use functions here to ensure invariance
        let s = core::any::TypeId::of::<fn(S) -> S>();
        if s == TypeId::of::<fn(T::S1) -> T::S1>() {
            unsafe { std::mem::transmute(self.t2()) }
        } else if s == TypeId::of::<fn(T::S2) -> T::S2>() {
            unsafe { std::mem::transmute(self.t1()) }
        } else {
            panic!(
                "type {} is not present in the service collection",
                type_name::<S>()
            );
        }
    }
}