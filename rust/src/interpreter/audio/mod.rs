

#[path = "audio-types.rs"]
mod audio_types;
#[path = "execute-audio-commands.rs"]
mod execute_audio_commands;
#[path = "build-audio-structure.rs"]
mod build_audio_structure;

pub use audio_types::{
    AudioHint, AudioStructure, Envelope, PlayMode, WaveformType,
};
pub use build_audio_structure::op_play;
pub use execute_audio_commands::{
    op_adsr, op_chord, op_fx_reset, op_gain, op_gain_reset, op_pan, op_pan_reset, op_saw,
    op_seq, op_sim, op_sine, op_slot, op_square, op_tri,
};

pub(crate) use audio_types::lookup_play_mode;

#[cfg(test)]
#[path = "audio-unit-tests.rs"]
mod audio_unit_tests;
#[cfg(test)]
#[path = "audio-integration-tests.rs"]
mod audio_integration_tests;
