//! `time_series.csv` writer. M3.
//!
//! Column list pinned by the implementation plan; column names are
//! stable. Negative `net_utility` is preserved by the comparison
//! aggregator, but the time-series file holds raw deltas only —
//! welfare is in `metrics_comparison.txt`.

use std::{io::Write, path::Path};

use anyhow::Result;

use super::collector::TimeSeriesRow;

/// Pinned column header. Metric names match `metrics_comparison.txt`
/// where overlapping (`fees_paid_lovelace`, `refund_lovelace`).
pub const HEADER: &str = "slot,c_priority,c_standard,util_priority_window_x_1e9,\
util_standard_window_x_1e9,mempool_bytes_total,mempool_bytes_priority,\
mempool_bytes_standard,included_bytes_priority,included_bytes_standard,\
included_count_priority,included_count_standard,evicted_quote_drift_count,\
fees_paid_lovelace,refund_lovelace";

pub fn write_csv(path: &Path, rows: &[TimeSeriesRow]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "{HEADER}")?;
    for r in rows {
        writeln!(
            f,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            r.slot,
            r.c_priority_quote_per_byte,
            r.c_standard_quote_per_byte,
            r.priority_window_util_x_1e9,
            r.standard_window_util_x_1e9,
            r.mempool_bytes_total,
            r.mempool_bytes_priority,
            r.mempool_bytes_standard,
            r.included_bytes_priority,
            r.included_bytes_standard,
            r.included_count_priority,
            r.included_count_standard,
            r.evicted_quote_drift_count,
            r.fees_paid_lovelace,
            r.refund_lovelace,
        )?;
    }
    Ok(())
}
