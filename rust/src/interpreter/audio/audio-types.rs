





use super::super::Interpreter;
use crate::types::ValueExt;
use serde::Serialize;





#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WaveformType {
    #[default]
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Envelope {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.0,
            sustain: 1.0,
            release: 0.01,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioHint {
    pub chord: bool,
    pub envelope: Option<Envelope>,
    pub waveform: WaveformType,
}

impl ValueExt for AudioHint {
    fn clone_box(&self) -> Box<dyn ValueExt> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}





pub(crate) struct MusicState {
    pub play_mode: PlayMode,
}

pub(crate) fn lookup_play_mode(interp: &Interpreter) -> PlayMode {
    interp
        .module_state
        .get("MUSIC")
        .and_then(|s| s.downcast_ref::<MusicState>())
        .map(|s| s.play_mode)
        .unwrap_or_default()
}

pub(crate) fn update_play_mode(interp: &mut Interpreter, mode: PlayMode) {
    if let Some(state) = interp.module_state.get_mut("MUSIC") {
        if let Some(ms) = state.downcast_mut::<MusicState>() {
            ms.play_mode = mode;
            return;
        }
    }
    interp.module_state.insert(
        "MUSIC".to_string(),
        Box::new(MusicState { play_mode: mode }),
    );
}






#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PlayMode {
    #[default]
    Sequential,
    Simultaneous,
}






#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum AudioStructure {
    #[serde(rename = "tone")]
    Tone {
        frequency: f64,
        duration: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        envelope: Option<Envelope>,
        #[serde(skip_serializing_if = "is_default_waveform")]
        waveform: WaveformType,
    },
    #[serde(rename = "rest")]
    Rest { duration: f64 },
    #[serde(rename = "seq")]
    Seq {
        children: Vec<AudioStructure>,
        #[serde(skip_serializing_if = "Option::is_none")]
        envelope: Option<Envelope>,
        #[serde(skip_serializing_if = "is_default_waveform")]
        waveform: WaveformType,
    },
    #[serde(rename = "sim")]
    Sim {
        children: Vec<AudioStructure>,
        #[serde(skip_serializing_if = "Option::is_none")]
        envelope: Option<Envelope>,
        #[serde(skip_serializing_if = "is_default_waveform")]
        waveform: WaveformType,
    },
}


pub(crate) fn is_default_waveform(wf: &WaveformType) -> bool {
    *wf == WaveformType::Sine
}





#[derive(Debug, Serialize)]
pub(crate) struct PlayCommand {
    #[serde(rename = "type")]
    pub command_type: String,
    pub structure: AudioStructure,
}
