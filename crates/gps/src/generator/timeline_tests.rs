use super::*;

#[test]
fn fractional_steps_preserve_long_run_sample_total() -> Result<(), Error> {
    let mut timeline = SampleTimeline::new(
        GpsTime { week: 1, sec: 0.0 },
        1_000_000.0,
        1.0 / 30.0,
        Some(1.0),
        None,
    )?;
    let mut sizes = Vec::new();
    while let Some(block) = timeline.next_block()? {
        sizes.push(block.sample_count);
    }
    assert_eq!(sizes.len(), 30);
    assert_eq!(sizes.iter().sum::<usize>(), 1_000_000);
    assert!(sizes.contains(&33_333));
    assert!(sizes.contains(&33_334));
    Ok(())
}

#[test]
fn block_is_split_at_exact_thirty_second_deadline() -> Result<(), Error> {
    let mut timeline = SampleTimeline::new(
        GpsTime {
            week: 10,
            sec: 29.95,
        },
        1_000_000.0,
        0.1,
        Some(0.1),
        None,
    )?;
    let first = timeline
        .next_block()?
        .ok_or_else(|| Error::msg("missing first timeline block"))?;
    let second = timeline
        .next_block()?
        .ok_or_else(|| Error::msg("missing second timeline block"))?;
    assert_eq!(first.sample_count, 50_000);
    assert_eq!(second.sample_count, 50_000);
    assert_eq!(first.step_start_sample, 0);
    assert_eq!(first.step_end_sample, 100_000);
    assert_eq!(first.end_sample, 50_000);
    assert!((first.step_endpoint_fraction() - 0.5).abs() < f64::EPSILON);
    assert!((second.step_endpoint_fraction() - 1.0).abs() < f64::EPSILON);
    assert_eq!(first.frame_deadlines, vec![GpsTime {
        week: 10,
        sec: 30.0,
    }]);
    assert!(second.frame_deadlines.is_empty());
    assert!(timeline.next_block()?.is_none());
    Ok(())
}

#[test]
fn clipped_step_retains_full_endpoint_fraction() -> Result<(), Error> {
    let mut timeline = SampleTimeline::new(
        GpsTime { week: 10, sec: 1.0 },
        10_000.0,
        2.0,
        Some(1.0),
        Some(1),
    )?;
    let block = timeline
        .next_block()?
        .ok_or_else(|| Error::msg("missing clipped timeline block"))?;
    assert_eq!(block.sample_count, 10_000);
    assert_eq!(block.step_start_sample, 0);
    assert_eq!(block.step_end_sample, 20_000);
    assert_eq!(block.end_sample, 10_000);
    assert!((block.step_endpoint_fraction() - 0.5).abs() < f64::EPSILON);
    Ok(())
}

#[test]
fn duration_rounds_to_nearest_sample_with_half_up_ties() -> Result<(), Error> {
    let start = GpsTime { week: 1, sec: 0.0 };
    let mut below_half = SampleTimeline::new(
        start.clone(),
        1_000_000.0,
        0.1,
        Some(0.000_000_49),
        None,
    )?;
    assert!(below_half.next_block()?.is_none());

    let mut exact_half =
        SampleTimeline::new(start, 1_000_000.0, 0.1, Some(0.000_000_5), None)?;
    let block = exact_half
        .next_block()?
        .ok_or_else(|| Error::msg("half-sample duration emitted no sample"))?;
    assert_eq!(block.sample_count, 1);
    assert!(exact_half.next_block()?.is_none());
    Ok(())
}

#[test]
fn frame_deadlines_cross_thirty_seconds_and_week_exactly() -> Result<(), Error>
{
    let start = GpsTime {
        week: 10,
        sec: SECONDS_IN_WEEK - 0.05,
    };
    let mut timeline =
        SampleTimeline::new(start.clone(), 1_000_000.0, 0.1, Some(0.1), None)?;
    let first = timeline
        .next_block()?
        .ok_or_else(|| Error::msg("missing first timeline block"))?;
    let second = timeline
        .next_block()?
        .ok_or_else(|| Error::msg("missing second timeline block"))?;
    let expected_end = start.add_secs(0.1);
    assert_eq!(first.end_time, GpsTime { week: 11, sec: 0.0 });
    assert_eq!(first.frame_deadlines, vec![GpsTime { week: 11, sec: 0.0 }]);
    assert_eq!(second.end_time.week, expected_end.week);
    assert!((second.end_time.sec - expected_end.sec).abs() < 1.0e-12);
    assert!(second.frame_deadlines.is_empty());
    Ok(())
}
