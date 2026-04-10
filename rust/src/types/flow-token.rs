use super::fraction::Fraction;
use super::{Value, ValueData};

use std::sync::atomic::{AtomicU64, Ordering};

static FLOW_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq)]
/// Internal runtime invariant token.
///
/// FlowToken is not a canonical user-facing value type. It exists for
/// diagnostic tracking, conservation verification, and optimization safety checks.
pub struct FlowToken {
    pub id: u64,
    pub total: Fraction,
    pub remaining: Fraction,
    pub shape: Vec<usize>,
    pub parent_flow_id: Option<u64>,
    pub child_flow_ids: Vec<u64>,
    pub mass_ratio: (u64, u64),
}

impl FlowToken {
    pub fn from_value(value: &Value) -> Self {
        let total: Fraction = Self::compute_value_total(value);
        let shape: Vec<usize> = value.shape();
        FlowToken {
            id: FLOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            total: total.clone(),
            remaining: total,
            shape,
            parent_flow_id: None,
            child_flow_ids: Vec::new(),
            mass_ratio: (1, 1),
        }
    }

    fn compute_value_total(value: &Value) -> Fraction {
        match &value.data {
            ValueData::Nil => Fraction::from(0),
            ValueData::Scalar(f) => {
                if f.is_nil() {
                    Fraction::from(0)
                } else {
                    f.clone()
                }
            }
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                let mut acc = Fraction::from(0);
                for child in v.iter() {
                    let child_total: Fraction = Self::compute_value_total(child);
                    acc = acc.add(&child_total.abs());
                }
                acc
            }
            ValueData::CodeBlock(_) | ValueData::ProcessHandle(_) | ValueData::SupervisorHandle(_) => Fraction::from(0),
        }
    }

    pub fn consume(
        &self,
        amount: &Fraction,
    ) -> std::result::Result<(Fraction, FlowToken), crate::error::AjisaiError> {
        if amount > &self.remaining {
            return Err(crate::error::AjisaiError::OverConsumption {
                requested: format!("{}", amount),
                remaining: format!("{}", self.remaining),
            });
        }
        let new_remaining: Fraction = self.remaining.sub(amount);
        Ok((
            amount.clone(),
            FlowToken {
                id: self.id,
                total: self.total.clone(),
                remaining: new_remaining,
                shape: self.shape.clone(),
                parent_flow_id: self.parent_flow_id,
                child_flow_ids: self.child_flow_ids.clone(),
                mass_ratio: self.mass_ratio,
            },
        ))
    }

    pub fn verify_conservation(
        &self,
        consumed: &[Fraction],
    ) -> std::result::Result<(), crate::error::AjisaiError> {
        let mut sum = Fraction::from(0);
        for c in consumed {
            sum = sum.add(&c.abs());
        }
        let reconstructed: Fraction = sum.add(&self.remaining);
        if reconstructed != self.total {
            return Err(crate::error::AjisaiError::FlowBreak {
                flow_id: self.id,
                reason: format!(
                    "conservation mismatch: total={} but consumed+remaining={}",
                    self.total, reconstructed
                ),
            });
        }
        Ok(())
    }

    pub fn assert_complete(
        &self,
        context: &str,
    ) -> std::result::Result<(), crate::error::AjisaiError> {
        if !self.remaining.is_zero() {
            return Err(crate::error::AjisaiError::UnconsumedLeak {
                remainder: format!("{}", self.remaining),
                context: context.to_string(),
            });
        }
        Ok(())
    }

    pub fn is_exhausted(&self) -> bool {
        self.remaining.is_zero()
    }

    pub fn is_reusable_allocation(&self) -> bool {
        self.remaining == self.total
            && self.parent_flow_id.is_none()
            && self.child_flow_ids.is_empty()
            && self.mass_ratio == (1, 1)
    }

    #[inline]
    pub fn can_update_in_place(&self, value: &Value) -> bool {
        self.is_reusable_allocation() && value.is_uniquely_owned()
    }

    pub fn bifurcate(
        &self,
        n: usize,
    ) -> std::result::Result<(FlowToken, Vec<FlowToken>), crate::error::AjisaiError> {
        if n == 0 {
            return Err(crate::error::AjisaiError::Custom(
                "Bifurcation requires at least 1 branch".to_string(),
            ));
        }
        if self.remaining.is_zero() {
            let children: Vec<FlowToken> = (0..n)
                .map(|_| FlowToken {
                    id: FLOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
                    total: Fraction::from(0),
                    remaining: Fraction::from(0),
                    shape: self.shape.clone(),
                    parent_flow_id: Some(self.id),
                    child_flow_ids: Vec::new(),
                    mass_ratio: (1, n as u64),
                })
                .collect();
            let child_ids: Vec<u64> = children.iter().map(|c| c.id).collect();
            let parent = FlowToken {
                id: self.id,
                total: self.total.clone(),
                remaining: Fraction::from(0),
                shape: self.shape.clone(),
                parent_flow_id: self.parent_flow_id,
                child_flow_ids: child_ids,
                mass_ratio: self.mass_ratio,
            };
            return Ok((parent, children));
        }

        let denom: Fraction = Fraction::from(n as i64);
        let child_mass: Fraction = self.remaining.div(&denom);

        let mut children = Vec::with_capacity(n);
        let mut child_ids = Vec::with_capacity(n);

        for _ in 0..n {
            let child_id: u64 = FLOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
            child_ids.push(child_id);
            children.push(FlowToken {
                id: child_id,
                total: child_mass.clone(),
                remaining: child_mass.clone(),
                shape: self.shape.clone(),
                parent_flow_id: Some(self.id),
                child_flow_ids: Vec::new(),
                mass_ratio: (1, n as u64),
            });
        }

        let parent = FlowToken {
            id: self.id,
            total: self.total.clone(),
            remaining: Fraction::from(0),
            shape: self.shape.clone(),
            parent_flow_id: self.parent_flow_id,
            child_flow_ids: child_ids,
            mass_ratio: self.mass_ratio,
        };

        Ok((parent, children))
    }

    pub fn verify_bifurcation_conservation(
        parent_remaining: &Fraction,
        children: &[FlowToken],
    ) -> std::result::Result<(), crate::error::AjisaiError> {
        let mut sum = Fraction::from(0);
        for child in children {
            sum = sum.add(&child.total);
        }
        if &sum != parent_remaining {
            return Err(crate::error::AjisaiError::BifurcationViolation {
                parent_mass: format!("{}", parent_remaining),
                children_sum: format!("{}", sum),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FlowResult {
    pub output: Value,
    pub remainder: FlowToken,
    pub consumed: Fraction,
}
