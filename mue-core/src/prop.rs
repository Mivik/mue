use crate::signal::{Access, ReadSignal};

#[derive(Clone, Copy)]
pub enum Prop<T> {
    Static(T),
    Dynamic(ReadSignal<T>),
}

impl<T: Default> Default for Prop<T> {
    fn default() -> Self {
        Self::Static(T::default())
    }
}

impl<T: Clone + 'static> Access for Prop<T> {
    type Value = T;

    fn get_clone(&self) -> T {
        match self {
            Self::Static(value) => value.clone(),
            Self::Dynamic(signal) => signal.get_clone(),
        }
    }

    fn get_clone_untracked(&self) -> T {
        match self {
            Self::Static(value) => value.clone(),
            Self::Dynamic(signal) => signal.get_clone_untracked(),
        }
    }
}

impl<T: 'static> Prop<T> {
    pub fn get_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Static(value) => Some(value),
            Self::Dynamic(_) => None,
        }
    }

    pub fn map<U: PartialEq + 'static>(&self, f: impl Fn(T) -> U + 'static) -> Prop<U>
    where
        T: Clone,
    {
        match self {
            Self::Static(value) => Prop::Static(f(value.clone())),
            Self::Dynamic(signal) => Prop::Dynamic(signal.map(f)),
        }
    }
}

impl<T> From<T> for Prop<T> {
    fn from(value: T) -> Self {
        Self::Static(value)
    }
}

impl<T> From<ReadSignal<T>> for Prop<T> {
    fn from(signal: ReadSignal<T>) -> Self {
        Self::Dynamic(signal)
    }
}

pub trait IntoProp<T> {
    fn into_prop(self) -> Prop<T>;
}

impl<T> IntoProp<T> for Prop<T> {
    fn into_prop(self) -> Prop<T> {
        self
    }
}

impl<T> IntoProp<T> for T {
    fn into_prop(self) -> Prop<T> {
        Prop::Static(self)
    }
}

impl<T> IntoProp<T> for ReadSignal<T> {
    fn into_prop(self) -> Prop<T> {
        Prop::Dynamic(self)
    }
}

impl<T> IntoProp<Option<T>> for T {
    fn into_prop(self) -> Prop<Option<T>> {
        Prop::Static(Some(self))
    }
}

#[macro_export]
macro_rules! default_props {
    (@extract_default ($ty:ty, $default:expr)) => {
        Prop::Static($default)
    };

    (@extract_default ($ty:ty, )) => {
        Prop::Static(<$ty>::default())
    };

    (
        $vis:vis struct $name:ident {
            $($field:ident:$ty:ty $(= $default:expr)?),* $(,)?
        }
    ) => {
        #[derive(Clone, Copy)]
        $vis struct $name {
            $(pub $field: Prop<$ty>),*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $($field: $crate::default_props!(@extract_default ($ty, $($default)?))),*
                }
            }
        }

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }

            $(
                pub fn $field(mut self, value: impl Into<Prop<$ty>>) -> Self {
                    self.$field = value.into();
                    self
                }
            )*
        }
    };
}
