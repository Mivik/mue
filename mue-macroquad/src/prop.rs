use std::cell::RefCell;

use mue_core::Prop;
use type_map::TypeMap;

thread_local! {
    static PROPS: RefCell<TypeMap> = RefCell::new(TypeMap::new());
}

pub trait WithProp<R> {
    fn with<T: 'static>(self, prop: impl Into<Prop<T>>) -> impl FnOnce() -> R;
}

impl<F, R> WithProp<R> for F
where
    F: FnOnce() -> R,
{
    fn with<T: 'static>(self, prop: impl Into<Prop<T>>) -> impl FnOnce() -> R {
        move || {
            let prev_prop = PROPS.with_borrow_mut(|m| m.insert(prop.into()));
            let result = self();
            if let Some(prop) = prev_prop {
                PROPS.with_borrow_mut(|m| m.insert(prop));
            }
            result
        }
    }
}

pub trait Bind<Args, R> {
    fn bind(self, value: Args) -> impl FnOnce() -> R;
}

macro_rules! impl_bind {
    ($($p:ident)*) => {
        #[allow(non_snake_case)]
        impl<F, R, $($p),*> Bind<($($p,)*), R> for F
        where
            F: FnOnce($($p),*) -> R,
        {
            fn bind(self, value: ($($p,)*)) -> impl FnOnce() -> R {
                move || {
                    let ($($p,)*) = value;
                    self($($p),*)
                }
            }
        }
    };
}

impl_bind!(T1);
impl_bind!(T1 T2);
impl_bind!(T1 T2 T3);
impl_bind!(T1 T2 T3 T4);
impl_bind!(T1 T2 T3 T4 T5);
impl_bind!(T1 T2 T3 T4 T5 T6);
impl_bind!(T1 T2 T3 T4 T5 T6 T7);
impl_bind!(T1 T2 T3 T4 T5 T6 T7 T8);

pub fn prop<T: 'static>() -> Option<Prop<T>> {
    PROPS.with_borrow_mut(|m| m.remove::<Prop<T>>())
}
pub fn prop_or<T: 'static>(default: T) -> Prop<T> {
    prop::<T>().unwrap_or(Prop::Static(default))
}
pub fn prop_or_else<T: 'static>(default: impl FnOnce() -> T) -> Prop<T> {
    prop::<T>().unwrap_or_else(|| Prop::Static(default()))
}
pub fn prop_or_default<T: Default + 'static>() -> Prop<T> {
    prop::<T>().unwrap_or_default()
}
