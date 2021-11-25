use std::cmp::Ordering;

use num_cmp::NumCmp;
use serde_json::{Number, Value};


pub trait PartialOrdering<Rhs: ?Sized = Self>: PartialEq<Rhs> {
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering>;

    #[inline]
    fn lt(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Less))
    }

    #[inline]
    fn le(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Less | Ordering::Equal))
    }

    #[inline]
    fn gt(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Greater))
    }

    #[inline]
    fn ge(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Greater | Ordering::Equal))
    }
}

impl PartialOrdering for Value {
    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        match self {
            Value::Number(first) => match other {
                Value::Number(second) => first.partial_cmp(second),
                _ => None,
            },
            Value::String(first) => match other {
                Value::String(second) => first.partial_cmp(second),
                _ => None,
            },
            Value::Bool(first) => match other {
                Value::Bool(second) => first.partial_cmp(second),
                _ => None,
            },
            Value::Null => match other {
                Value::Null => Some(Ordering::Equal),
                _ => None,
            },
            Value::Array(first) => match other {
                Value::Array(second) => first.partial_cmp(second),
                _ => None,
            },
            Value::Object(..) => None,
        }
    }
}

impl PartialOrdering for Vec<Value> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {

        let left = self;
        let right = other;
        let l = std::cmp::min(left.len(), right.len());

        // Slice to the loop iteration range to enable bound check
        // elimination in the compiler
        let lhs = &left[..l];
        let rhs = &right[..l];

        for i in 0..l {
            match lhs[i].partial_cmp(&rhs[i]) {
                Some(Ordering::Equal) => (),
                non_eq => return non_eq,
            }
        }

        left.len().partial_cmp(&right.len())

    }
}

impl PartialOrdering for Number {
    fn partial_cmp(&self, other: &Number) -> Option<Ordering> {
        if let Some(first) = self.as_f64() {
            if let Some(second) = other.as_f64() {
                NumCmp::num_cmp(first, second)
            } else if let Some(second) = other.as_u64() {
                NumCmp::num_cmp(first, second)
            } else if let Some(second) = other.as_i64() {
                NumCmp::num_cmp(first, second)
            } else {
                None
            } 
        } else if let Some(first) = self.as_u64() {
            if let Some(second) = other.as_f64() {
                NumCmp::num_cmp(first, second)
            } else if let Some(second) = other.as_u64() {
                NumCmp::num_cmp(first, second)
            } else if let Some(second) = other.as_i64() {
                NumCmp::num_cmp(first, second)
            } else {
                None
            }
        } else if let Some(first) = self.as_i64() {
            if let Some(second) = other.as_f64() {
                NumCmp::num_cmp(first, second)
            } else if let Some(second) = other.as_u64() {
                NumCmp::num_cmp(first, second)
            } else if let Some(second) = other.as_i64() {
                NumCmp::num_cmp(first, second)
            } else {
                None
            }
        } else {
            None
        }

    }
}
