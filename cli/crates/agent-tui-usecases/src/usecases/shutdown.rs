//! Shutdown use case.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use crate::domain::ShutdownInput;
use crate::domain::ShutdownOutput;
use crate::usecases::ports::ShutdownNotifier;

pub trait ShutdownUseCase: Send + Sync {
    fn execute(&self, input: ShutdownInput) -> ShutdownOutput;
}

pub struct ShutdownUseCaseImpl {
    shutdown_flag: Arc<AtomicBool>,
    notifier: Arc<dyn ShutdownNotifier>,
}

impl ShutdownUseCaseImpl {
    pub fn new(shutdown_flag: Arc<AtomicBool>, notifier: Arc<dyn ShutdownNotifier>) -> Self {
        Self {
            shutdown_flag,
            notifier,
        }
    }
}

impl ShutdownUseCase for ShutdownUseCaseImpl {
    fn execute(&self, _input: ShutdownInput) -> ShutdownOutput {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        let acknowledged = self.notifier.notify().is_ok();

        ShutdownOutput { acknowledged }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    struct FailingShutdownNotifier;

    impl crate::usecases::ports::ShutdownNotifier for FailingShutdownNotifier {
        fn notify(&self) -> Result<(), io::Error> {
            Err(io::Error::other("wakeup pipe closed"))
        }
    }

    #[test]
    fn test_shutdown_usecase_sets_flag_to_true() {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let usecase = ShutdownUseCaseImpl::new(
            Arc::clone(&shutdown_flag),
            Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier),
        );

        assert!(!shutdown_flag.load(Ordering::SeqCst));

        let output = usecase.execute(ShutdownInput);

        assert!(shutdown_flag.load(Ordering::SeqCst));
        assert!(output.acknowledged);
    }

    #[test]
    fn test_shutdown_usecase_returns_acknowledged_true() {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let usecase = ShutdownUseCaseImpl::new(
            shutdown_flag,
            Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier),
        );

        let output = usecase.execute(ShutdownInput);

        assert!(output.acknowledged);
    }

    #[test]
    fn test_shutdown_usecase_returns_acknowledged_false_when_notify_fails() {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let usecase = ShutdownUseCaseImpl::new(
            Arc::clone(&shutdown_flag),
            Arc::new(FailingShutdownNotifier),
        );

        let output = usecase.execute(ShutdownInput);

        assert!(shutdown_flag.load(Ordering::SeqCst));
        assert!(!output.acknowledged);
    }

    #[test]
    fn test_shutdown_usecase_is_idempotent() {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let usecase = ShutdownUseCaseImpl::new(
            Arc::clone(&shutdown_flag),
            Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier),
        );

        let output1 = usecase.execute(ShutdownInput);
        let output2 = usecase.execute(ShutdownInput);

        assert!(output1.acknowledged);
        assert!(output2.acknowledged);
        assert!(shutdown_flag.load(Ordering::SeqCst));
    }
}
