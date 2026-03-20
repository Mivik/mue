use crate::node::NodeInner;

pub(crate) struct HookFn<T> {
    callback: Option<Box<dyn Fn(&T)>>,
}

impl<T> Default for HookFn<T> {
    fn default() -> Self {
        Self { callback: None }
    }
}

impl<T: 'static> HookFn<T> {
    pub fn is_empty(&self) -> bool {
        self.callback.is_none()
    }

    pub fn append(&mut self, callback: impl Fn(&T) + 'static) {
        if let Some(existing) = self.callback.take() {
            self.callback = Some(Box::new(move |arg| {
                existing(arg);
                callback(arg);
            }));
        } else {
            self.callback = Some(Box::new(callback));
        }
    }

    pub fn extend(&mut self, other: Self) {
        if let Some(other_callback) = other.callback {
            self.append(other_callback);
        }
    }

    pub fn invoke(&self, arg: &T) {
        if let Some(callback) = &self.callback {
            callback(arg);
        }
    }
}

#[derive(Default)]
pub(crate) struct Hooks {
    pub render: HookFn<()>,
}

pub fn on_render(callback: impl Fn(&()) + 'static) {
    NodeInner::with_mut(|node| node.hooks.render.append(callback));
}
