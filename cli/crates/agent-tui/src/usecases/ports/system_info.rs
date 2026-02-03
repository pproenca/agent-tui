pub trait SystemInfoProvider: Send + Sync {
    fn pid(&self) -> u32;
    fn uptime_ms(&self) -> u64;
    fn version(&self) -> String;
    fn commit(&self) -> String;
}
