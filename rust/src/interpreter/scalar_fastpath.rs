use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};

pub(crate) enum ScalarFastWrap {
    Scalar,
    Tensor(Vec<usize>),
}

pub(crate) struct ScalarFastOperand {
    pub(crate) fraction: Fraction,
    pub(crate) wrap: ScalarFastWrap,
}

pub(crate) fn scalar_fast_operand(value: &Value) -> Option<ScalarFastOperand> {
    match &value.data {
        ValueData::Scalar(f) => Some(ScalarFastOperand {
            fraction: f.clone(),
            wrap: ScalarFastWrap::Scalar,
        }),
        ValueData::Tensor { data, shape } if data.len() == 1 => Some(ScalarFastOperand {
            fraction: data.get_small_fraction(0)?,
            wrap: ScalarFastWrap::Tensor((**shape).clone()),
        }),
        ValueData::Vector(children)
            if value.hint != Interpretation::Text && children.len() == 1 =>
        {
            let child = scalar_fast_operand(&children[0])?;
            let mut shape = Vec::with_capacity(2);
            shape.push(1);
            match child.wrap {
                ScalarFastWrap::Scalar => {}
                ScalarFastWrap::Tensor(child_shape) => shape.extend(child_shape),
            }
            Some(ScalarFastOperand {
                fraction: child.fraction,
                wrap: ScalarFastWrap::Tensor(shape),
            })
        }
        _ => None,
    }
}

pub(crate) fn same_scalar_fast_wrap(a: &ScalarFastWrap, b: &ScalarFastWrap) -> bool {
    match (a, b) {
        (ScalarFastWrap::Scalar, ScalarFastWrap::Scalar) => true,
        (ScalarFastWrap::Tensor(a_shape), ScalarFastWrap::Tensor(b_shape)) => a_shape == b_shape,
        _ => false,
    }
}
