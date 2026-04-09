




use super::audio_types::{update_play_mode, PlayMode, WaveformType};
use super::super::Interpreter;
use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};
use num_traits::ToPrimitive;






pub fn op_seq(interp: &mut Interpreter) -> Result<()> {
    update_play_mode(interp, PlayMode::Sequential);
    Ok(())
}






pub fn op_sim(interp: &mut Interpreter) -> Result<()> {
    update_play_mode(interp, PlayMode::Simultaneous);
    Ok(())
}






fn extract_scalar_from_value(val: &Value) -> Option<Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Some(f.clone()),
        ValueData::Vector(children) if children.len() == 1 => extract_scalar_from_value(&children[0]),
        _ => None,
    }
}




pub fn op_slot(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;


    let frac =
        extract_scalar_from_value(&val).ok_or_else(|| AjisaiError::from("SLOT requires a single number"))?;


    let seconds = frac
        .to_f64()
        .ok_or_else(|| AjisaiError::from("SLOT value too large"))?;


    if seconds <= 0.0 {
        return Err(AjisaiError::from("SLOT duration must be positive"));
    }


    if seconds < 0.01 {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@SLOT duration {}s is very short\n",
            seconds
        ));
    }
    if seconds > 10.0 {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@SLOT duration {}s is very long\n",
            seconds
        ));
    }


    interp
        .output_buffer
        .push_str(&format!("CONFIG:{{\"slot_duration\":{}}}\n", seconds));

    Ok(())
}








pub fn op_gain(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let frac = extract_scalar_from_value(&val).ok_or_else(|| AjisaiError::from("GAIN requires a number"))?;

    let gain = frac
        .to_f64()
        .ok_or_else(|| AjisaiError::from("GAIN value too large"))?;


    let clamped = gain.clamp(0.0, 1.0);

    if (gain - clamped).abs() > f64::EPSILON {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@GAIN {} clamped to {}\n",
            gain, clamped
        ));
    }


    interp
        .output_buffer
        .push_str(&format!("EFFECT:{{\"gain\":{}}}\n", clamped));

    Ok(())
}


pub fn op_gain_reset(interp: &mut Interpreter) -> Result<()> {
    interp.output_buffer.push_str("EFFECT:{\"gain\":1.0}\n");
    Ok(())
}








pub fn op_pan(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let frac = extract_scalar_from_value(&val).ok_or_else(|| AjisaiError::from("PAN requires a number"))?;

    let pan = frac
        .to_f64()
        .ok_or_else(|| AjisaiError::from("PAN value too large"))?;


    let clamped = pan.clamp(-1.0, 1.0);

    if (pan - clamped).abs() > f64::EPSILON {
        interp.output_buffer.push_str(&format!(
            "Warning: MUSIC@PAN {} clamped to {}\n",
            pan, clamped
        ));
    }


    interp
        .output_buffer
        .push_str(&format!("EFFECT:{{\"pan\":{}}}\n", clamped));

    Ok(())
}


pub fn op_pan_reset(interp: &mut Interpreter) -> Result<()> {
    interp.output_buffer.push_str("EFFECT:{\"pan\":0.0}\n");
    Ok(())
}


pub fn op_fx_reset(interp: &mut Interpreter) -> Result<()> {
    interp
        .output_buffer
        .push_str("EFFECT:{\"gain\":1.0,\"pan\":0.0}\n");
    Ok(())
}






pub fn op_chord(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;


    if !val.is_vector() {
        return Err(AjisaiError::from("CHORD requires a vector"));
    }



    interp.stack.push(val);
    Ok(())
}








pub fn op_adsr(interp: &mut Interpreter) -> Result<()> {

    let params = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;


    let target = interp.stack.pop().ok_or_else(|| {

        interp.stack.push(params.clone());
        AjisaiError::StackUnderflow
    })?;


    let param_children = match &params.data {
        ValueData::Vector(v) => v,
        _ => {
            interp.stack.push(target);
            interp.stack.push(params);
            return Err(AjisaiError::from("ADSR parameters must be a vector"));
        }
    };


    if param_children.len() != 4 {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from(
            "ADSR requires exactly 4 values: [attack decay sustain release]",
        ));
    }


    if !target.is_vector() {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from("ADSR target must be a vector"));
    }


    let attack = param_children[0]
        .as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR attack must be a number"))?;
    let decay = param_children[1]
        .as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR decay must be a number"))?;
    let sustain = param_children[2]
        .as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR sustain must be a number"))?;
    let release = param_children[3]
        .as_scalar()
        .and_then(|f| f.to_f64())
        .ok_or_else(|| AjisaiError::from("ADSR release must be a number"))?;


    if attack < 0.0 || decay < 0.0 || release < 0.0 {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from("ADSR times must be non-negative"));
    }
    if sustain < 0.0 || sustain > 1.0 {
        interp.stack.push(target);
        interp.stack.push(params);
        return Err(AjisaiError::from(
            "ADSR sustain must be between 0.0 and 1.0",
        ));
    }



    interp.stack.push(target);
    Ok(())
}






pub fn op_sine(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Sine)
}


pub fn op_square(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Square)
}


pub fn op_saw(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Sawtooth)
}


pub fn op_tri(interp: &mut Interpreter) -> Result<()> {
    apply_waveform(interp, WaveformType::Triangle)
}


fn apply_waveform(interp: &mut Interpreter, _waveform: WaveformType) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;


    if !val.is_vector() {
        return Err(AjisaiError::from("Waveform word requires a vector"));
    }



    interp.stack.push(val);
    Ok(())
}
