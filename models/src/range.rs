use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

fn option_cmp<T: PartialOrd + Ord>(first: Option<&T>, second: Option<&T>) -> Ordering {
    match first {
        Some(t) => match second {
            Some(o) => t.cmp(o),
            None => Ordering::Less,
        },
        None => match second {
            Some(o) => Ordering::Greater,
            None => Ordering::Equal,
        },
    }
}

pub trait Range<T: PartialOrd + Eq + Ord>: Clone + Sized {
    fn start(&self) -> T;
    fn end(&self) -> Option<T>;
    fn new_range(start: &T, end: &Option<T>) -> Self
    where
        Self: Sized;
    fn contains(&self, ot: Option<&T>) -> bool {
        match ot {
            Some(t) => match self.end() {
                Some(et) => t < &et && t >= &self.start(),
                None => t >= &self.start(),
            },
            None => false,
        }
    }

    fn overlaps(&self, r2: &Self) -> bool {
        self.contains(Some(&r2.start())) || r2.contains(Some(&self.start()))
    }

    fn intersection(&self, other: &Self) -> Option<Self> {
        if self.start() <= other.start()
            && option_cmp(self.end().as_ref(), other.end().as_ref()) == Ordering::Greater
        {
            Some(other.clone())
        } else if self.start() >= other.start()
            && option_cmp(self.end().as_ref(), other.end().as_ref()) == Ordering::Less
        {
            Some(self.clone())
        } else {
            let lower_bound = if self.start() >= other.start() {
                self.start()
            } else {
                other.start()
            };
            let upper_bound =
                if option_cmp(self.end().as_ref(), other.end().as_ref()) == Ordering::Less {
                    self.end()
                } else {
                    other.end()
                };

            if option_cmp(Some(&lower_bound), upper_bound.as_ref()) == Ordering::Less {
                Some(Range::new_range(&lower_bound, &upper_bound))
            } else {
                None
            }
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct ClosedRange<T: PartialOrd + Eq + Clone + Ord> {
    pub start: T,
    pub end: T,
}

impl<T: PartialOrd + Ord + Eq + Clone> Range<T> for ClosedRange<T> {
    fn start(&self) -> T {
        self.start.clone()
    }
    fn end(&self) -> Option<T> {
        Some(self.end.clone())
    }
    /// Use new_closed_range instead
    fn new_range(start: &T, end: &Option<T>) -> ClosedRange<T> {
        match end {
            Some(e) => ClosedRange {
                start: start.clone(),
                end: e.clone(),
            },
            None => panic!("Bound Required for Closed Range"),
        }
    }
}

impl<T: PartialOrd + Ord + Eq + Clone> ClosedRange<T> {
    pub fn new_closed_range(start: &T, end: &T) -> ClosedRange<T> {
        ClosedRange::<T>::new_range(start, &Some(end.clone()))
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct OpenRange<T: PartialOrd + Ord + Eq + Clone> {
    pub start: T,
    pub end: Option<T>,
}

impl<T: PartialOrd + Ord + Eq + Clone> Range<T> for OpenRange<T> {
    fn start(&self) -> T {
        self.start.clone()
    }
    fn end(&self) -> Option<T> {
        self.end.clone()
    }
    fn new_range(start: &T, end: &Option<T>) -> OpenRange<T> {
        OpenRange {
            start: start.clone(),
            end: end.clone(),
        }
    }
}

impl<T: PartialOrd + Ord + Eq + Clone> OpenRange<T> {
    pub fn new_open_range(start: &T, end: &Option<T>) -> OpenRange<T> {
        OpenRange::new_range(start, end)
    }
}
