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
        !matches!(self.partial_cmp(other), Some(Ordering::Less | Ordering::Equal))
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
        if self.len() == other.len() {
            for i in 0..self.len() {
                let cmp = self[i].partial_cmp(&other[i]);
                if cmp != Some(Ordering::Equal) {
                    return cmp;
                }
            }
            Some(Ordering::Equal)
        } else {
            None
        }
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
