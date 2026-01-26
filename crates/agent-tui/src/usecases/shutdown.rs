use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{ShutdownInput, ShutdownOutput};

pub trait ShutdownUseCase: Send + Sync {
    fn execute(&self, input: ShutdownInput) -> ShutdownOutput;
}

pub struct ShutdownUseCaseImpl {
    shutdown_flag: Arc<AtomicBool>,
}

impl ShutdownUseCaseImpl {
    pub fn new(shutdown_flag: Arc<AtomicBool>) -> Self {
        Self { shutdown_flag }
    }
}

impl ShutdownUseCase for ShutdownUseCaseImpl {
    #[tracing::instrument(skip(self, _input))]
    fn execute(&self, _input: ShutdownInput) -> ShutdownOutput {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        ShutdownOutput { acknowledged: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_usecase_sets_flag_to_true() {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let usecase = ShutdownUseCaseImpl::new(Arc::clone(&shutdown_flag));

        assert!(!shutdown_flag.load(Ordering::SeqCst));

        let output = usecase.execute(ShutdownInput);

        assert!(shutdown_flag.load(Ordering::SeqCst));
        assert!(output.acknowledged);
    }

    #[test]
    fn test_shutdown_usecase_returns_acknowledged_true() {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let usecase = ShutdownUseCaseImpl::new(shutdown_flag);

        let output = usecase.execute(ShutdownInput);

        assert!(output.acknowledged);
    }

    #[test]
    fn test_shutdown_usecase_is_idempotent() {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let usecase = ShutdownUseCaseImpl::new(Arc::clone(&shutdown_flag));

        let output1 = usecase.execute(ShutdownInput);
        let output2 = usecase.execute(ShutdownInput);

        assert!(output1.acknowledged);
        assert!(output2.acknowledged);
        assert!(shutdown_flag.load(Ordering::SeqCst));
    }
}
