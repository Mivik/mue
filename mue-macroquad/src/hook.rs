use crate::node::NodeContext;

#[derive(Default)]
pub(crate) struct HookFn<T> {
    callback: Option<Box<dyn Fn(&T)>>,
}

impl<T: 'static> HookFn<T> {
    pub fn new() -> Self {
        Self { callback: None }
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
    NodeContext::with_mut(|ctx| ctx.hooks.render.append(callback));
}
