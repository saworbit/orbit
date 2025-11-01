/*!
 * Bandwidth throttling utilities
 */

use std::thread;
use std::time::{Duration, Instant};

/// Apply bandwidth limiting to slow down transfer rate
///
/// # Arguments
/// * `bytes_written` - Number of bytes written in this chunk
/// * `max_bandwidth` - Maximum bytes per second allowed
/// * `last_check` - Timestamp of last bandwidth check (updated by this function)
pub fn apply_limit(bytes_written: u64, max_bandwidth: u64, last_check: &mut Instant) {
    let elapsed = last_check.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();

    if elapsed_secs < 1.0 {
        let bytes_per_sec = bytes_written as f64 / elapsed_secs;
        if bytes_per_sec > max_bandwidth as f64 {
            let sleep_time = Duration::from_secs_f64(
                (bytes_written as f64 / max_bandwidth as f64) - elapsed_secs
            );
            thread::sleep(sleep_time);
        }
    }

    if elapsed >= Duration::from_secs(1) {
        *last_check = Instant::now();
    }
}
