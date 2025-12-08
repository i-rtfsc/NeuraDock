#[cfg(test)]
mod tests {
    use super::super::value_objects::*;

    #[test]
    fn test_balance_new_calculates_total_income_correctly() {
        let balance = Balance::new(100.0, 50.0);

        assert_eq!(balance.current_balance, 100.0);
        assert_eq!(balance.total_consumed, 50.0);
        assert_eq!(balance.total_income, 150.0); // 100 + 50
    }

    #[test]
    fn test_balance_with_zero_consumed() {
        let balance = Balance::new(100.0, 0.0);

        assert_eq!(balance.current_balance, 100.0);
        assert_eq!(balance.total_consumed, 0.0);
        assert_eq!(balance.total_income, 100.0);
    }

    #[test]
    fn test_balance_with_zero_current() {
        let balance = Balance::new(0.0, 50.0);

        assert_eq!(balance.current_balance, 0.0);
        assert_eq!(balance.total_consumed, 50.0);
        assert_eq!(balance.total_income, 50.0);
    }

    #[test]
    fn test_balance_with_all_zeros() {
        let balance = Balance::new(0.0, 0.0);

        assert_eq!(balance.current_balance, 0.0);
        assert_eq!(balance.total_consumed, 0.0);
        assert_eq!(balance.total_income, 0.0);
    }

    #[test]
    fn test_balance_with_large_numbers() {
        let balance = Balance::new(999999.99, 123456.78);

        assert_eq!(balance.current_balance, 999999.99);
        assert_eq!(balance.total_consumed, 123456.78);
        assert_eq!(balance.total_income, 1123456.77);
    }

    #[test]
    fn test_balance_with_floating_point_precision() {
        let balance = Balance::new(10.5, 20.3);

        // Use approximate comparison for floating point
        assert!((balance.total_income - 30.8).abs() < 0.0001);
    }

    #[test]
    fn test_check_in_status_variants() {
        // Test all variants exist and are distinct
        assert_ne!(CheckInStatus::Pending, CheckInStatus::Running);
        assert_ne!(CheckInStatus::Running, CheckInStatus::Completed);
        assert_ne!(CheckInStatus::Completed, CheckInStatus::Failed);
        assert_ne!(CheckInStatus::Failed, CheckInStatus::Cancelled);
        assert_ne!(CheckInStatus::Cancelled, CheckInStatus::Pending);
    }

    #[test]
    fn test_check_in_status_clone() {
        let status = CheckInStatus::Running;
        let cloned = status.clone();

        assert_eq!(status, cloned);
    }

    #[test]
    fn test_check_in_result_success_with_balance() {
        let balance = Balance::new(100.0, 50.0);
        let result = CheckInResult {
            success: true,
            balance: Some(balance.clone()),
            message: Some("Check-in successful".to_string()),
        };

        assert!(result.success);
        assert!(result.balance.is_some());
        assert_eq!(result.balance.as_ref().unwrap().current_balance, 100.0);
        assert_eq!(result.message.as_ref().unwrap(), "Check-in successful");
    }

    #[test]
    fn test_check_in_result_failure_without_balance() {
        let result = CheckInResult {
            success: false,
            balance: None,
            message: Some("Network error".to_string()),
        };

        assert!(!result.success);
        assert!(result.balance.is_none());
        assert_eq!(result.message.as_ref().unwrap(), "Network error");
    }

    #[test]
    fn test_check_in_result_success_without_message() {
        let balance = Balance::new(75.0, 25.0);
        let result = CheckInResult {
            success: true,
            balance: Some(balance),
            message: None,
        };

        assert!(result.success);
        assert!(result.balance.is_some());
        assert!(result.message.is_none());
    }

    #[test]
    fn test_balance_clone() {
        let balance = Balance::new(100.0, 50.0);
        let cloned = balance.clone();

        assert_eq!(cloned.current_balance, balance.current_balance);
        assert_eq!(cloned.total_consumed, balance.total_consumed);
        assert_eq!(cloned.total_income, balance.total_income);
    }
}
