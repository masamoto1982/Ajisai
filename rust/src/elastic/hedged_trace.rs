#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HedgedPath {
    Quantized,
    Plain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HedgedTraceEvent {
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct HedgedTrace {
    pub events: Vec<HedgedTraceEvent>,
}

impl HedgedTrace {
    pub fn push(&mut self, label: impl Into<String>, detail: impl Into<String>) {
        self.events.push(HedgedTraceEvent {
            label: label.into(),
            detail: detail.into(),
        });
    }
}
