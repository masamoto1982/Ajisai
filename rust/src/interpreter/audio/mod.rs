

mod audio_types;
mod execute_audio_commands;
mod build_audio_structure;
mod music_group;

pub use audio_types::{
    AudioHint, AudioStructure, Envelope, PlayMode, WaveformType,
};
pub use build_audio_structure::op_play;
pub use execute_audio_commands::{
    op_adsr, op_chord, op_explain, op_fx_reset, op_gain, op_gain_reset, op_pan, op_pan_reset,
    op_saw, op_seq, op_seq_group, op_sim, op_sim_group, op_sine, op_slot, op_square, op_tri,
};

pub(crate) use audio_types::lookup_play_mode;

#[cfg(test)]
mod audio_unit_tests;
#[cfg(test)]
mod audio_integration_tests;
