use std::sync::Arc;

pub trait ShutdownNotifier: Send + Sync {
    fn notify(&self);
}

#[derive(Default)]
pub struct NoopShutdownNotifier;

impl ShutdownNotifier for NoopShutdownNotifier {
    fn notify(&self) {}
}

pub type ShutdownNotifierHandle = Arc<dyn ShutdownNotifier>;
