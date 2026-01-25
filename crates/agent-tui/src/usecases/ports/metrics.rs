pub trait MetricsProvider: Send + Sync {
    fn requests(&self) -> u64;
    fn errors(&self) -> u64;
    fn lock_timeouts(&self) -> u64;
    fn poison_recoveries(&self) -> u64;
}
