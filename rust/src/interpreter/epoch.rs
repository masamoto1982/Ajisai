#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EpochSnapshot {
    pub global_epoch: u64,
    pub dictionary_epoch: u64,
    pub module_epoch: u64,
    pub execution_epoch: u64,
}
