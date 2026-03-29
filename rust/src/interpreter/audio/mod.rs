// rust/src/interpreter/audio/mod.rs
//
// 【責務】
// 音楽DSLの実装。SEQ（順次再生）とSIM（同時再生）の2つの操作で音楽を表現する。
// 分数システム（周波数/音長）を活用し、ベクタの要素を音声として再生する。
//
// 【分数の解釈】
// - n/d = nHz を dスロット再生
// - n = n/1 の省略（nHz を 1スロット）
// - 0/d = dスロット休符
//
// 【値の種類と動作】
// - 整数 n: nHz を 1スロット再生
// - 分数 n/d: nHz を dスロット再生
// - 0/d: dスロット休符
// - NIL: 1スロット休符
// - 文字列: Outputに出力（歌詞）、時間消費なし
//
// 【オペレーションターゲット】
// - . MUSIC@PLAY（デフォルト）: スタックトップを再生
// - .. MUSIC@PLAY: スタック全体を再生（マルチトラック）

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
