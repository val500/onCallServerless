pub mod call;
pub mod range;
pub mod schedule;
pub mod time;
pub mod users;

#[cfg(test)]
mod tests {
    use crate::range::{ClosedRange, OpenRange, Range};

    #[test]
    fn test_closed_range() {
        let r1: ClosedRange<i64> = ClosedRange::new_closed_range(&1, &5);
        assert_eq!(r1, ClosedRange { start: 1, end: 5 });

        //Within r1
        let r2: ClosedRange<i64> = ClosedRange::new_closed_range(&2, &4);
        assert_eq!(r2, ClosedRange { start: 2, end: 4 });
        assert!(r1.contains(Some(&2)));
        assert!(r1.overlaps(&r2));
        assert!(r2.overlaps(&r1));

        let r2 = ClosedRange::new_closed_range(&3, &6);
        assert!(r1.overlaps(&r2));
        assert!(r2.overlaps(&r1));

        let r2 = ClosedRange::new_closed_range(&0, &2);
        assert!(r1.overlaps(&r2));
        assert!(r2.overlaps(&r1));

        let r2 = ClosedRange::new_closed_range(&1, &5);
        assert!(r1.overlaps(&r2));
        assert!(r2.overlaps(&r1));

        let r2 = ClosedRange::new_closed_range(&6, &10);
        assert!(!r1.overlaps(&r2));
        assert!(!r2.overlaps(&r1));

        let r2 = ClosedRange::new_closed_range(&5, &10);
        assert!(!r1.overlaps(&r2));
        assert!(!r2.overlaps(&r1));
    }
}
