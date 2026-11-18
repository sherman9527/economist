pub fn is_cst_30m_rotation_slot(
    weekday_mon0: u32,
    hour: u32,
    minute: u32,
) -> bool {
    if weekday_mon0 >= 5 {
        return false;
    }
    if minute != 0 && minute != 30 {
        return false;
    }
    let total_min = hour * 60 + minute;
    // Morning session: 09:30..=11:30 (570..=690); Afternoon session: 13:00..=15:00 (780..=900)
    (570..=690).contains(&total_min) || (780..=900).contains(&total_min)
}

/// Maps a CST timestamp into its 30-minute rotation slot bucket with a 5-minute grace window
/// (`00..=04` or `30..=34`) so a slow network scan never causes the 20s poll loop to skip a slot.
pub fn cst_30m_rotation_slot_bucket(
    weekday_mon0: u32,
    hour: u32,
    minute: u32,
) -> Option<&'static str> {
    if weekday_mon0 >= 5 {
        return None;
    }
    match (hour, minute) {
        (9, 30..=34) => Some("09:30"),
        (10, 0..=4) => Some("10:00"),
        (10, 30..=34) => Some("10:30"),
        (11, 0..=4) => Some("11:00"),
        (11, 30..=34) => Some("11:30"),
        (13, 0..=4) => Some("13:00"),
        (13, 30..=34) => Some("13:30"),
        (14, 0..=4) => Some("14:00"),
        (14, 30..=34) => Some("14:30"),
        (15, 0..=4) => Some("15:00"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_exact_30m_ashare_trading_slots_mon_to_fri() {
        assert!(is_cst_30m_rotation_slot(0, 9, 30));
        assert!(is_cst_30m_rotation_slot(2, 11, 30));
        assert!(is_cst_30m_rotation_slot(4, 13, 0));
        assert!(is_cst_30m_rotation_slot(4, 14, 30));
        assert!(is_cst_30m_rotation_slot(4, 15, 0));

        assert!(!is_cst_30m_rotation_slot(0, 12, 0)); // lunch break
        assert!(!is_cst_30m_rotation_slot(5, 10, 0)); // Saturday
        assert!(!is_cst_30m_rotation_slot(1, 10, 15)); // non-30m boundary

        assert_eq!(cst_30m_rotation_slot_bucket(0, 9, 31), Some("09:30"));
        assert_eq!(cst_30m_rotation_slot_bucket(0, 11, 34), Some("11:30"));
        assert_eq!(cst_30m_rotation_slot_bucket(0, 12, 1), None);
        assert_eq!(cst_30m_rotation_slot_bucket(5, 9, 30), None);
    }
}
