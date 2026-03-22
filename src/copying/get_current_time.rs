use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_current_time() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("time should be after UNIX EPOCH");
    since_the_epoch.as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_get_current_time_called_then_it_should_return_millis() {
        let time = get_current_time();

        assert!(time > 0);
    }
}
