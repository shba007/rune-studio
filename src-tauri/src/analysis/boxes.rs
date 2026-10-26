use super::report::Finding;

pub type Box4 = [f32; 4];

pub const MIN_SIDE: f32 = 8.0;
pub const MAX_AREA_SHARE: f32 = 0.60;
pub const OVERRUN_TOLERANCE: f32 = 40.0;
pub const TILING_SHARE: f32 = 0.95;
const DUPLICATE_IOU: f32 = 0.98;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    Malformed,
    OutOfRange,
    TooSmall,
    TooLarge,
    Duplicate,
    TilesTheScreen,
}

impl Reject {
    pub fn why(self) -> &'static str {
        match self {
            Reject::Malformed => "not four finite numbers",
            Reject::OutOfRange => "outside the screen by more than the tolerance",
            Reject::TooSmall => "smaller than 8 units on a side",
            Reject::TooLarge => "larger than 60 percent of the screen",
            Reject::Duplicate => "the same box as another finding",
            Reject::TilesTheScreen => "one of a set of boxes that tile the whole screen",
        }
    }
}

pub fn usable_box(raw: [f32; 4]) -> Result<Box4, Reject> {
    if raw.iter().any(|value| !value.is_finite()) {
        return Err(Reject::Malformed);
    }
    let (lo, hi) = (-OVERRUN_TOLERANCE, 1000.0 + OVERRUN_TOLERANCE);
    if raw.iter().any(|value| *value < lo || *value > hi) {
        return Err(Reject::OutOfRange);
    }
    let clamp = |value: f32| value.clamp(0.0, 1000.0);
    let rect = [
        clamp(raw[0].min(raw[2])),
        clamp(raw[1].min(raw[3])),
        clamp(raw[0].max(raw[2])),
        clamp(raw[1].max(raw[3])),
    ];
    let (w, h) = (rect[2] - rect[0], rect[3] - rect[1]);
    if w < MIN_SIDE || h < MIN_SIDE {
        return Err(Reject::TooSmall);
    }
    if w * h > MAX_AREA_SHARE * 1_000_000.0 {
        return Err(Reject::TooLarge);
    }
    Ok(rect)
}

fn iou(a: Box4, b: Box4) -> f32 {
    let w = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let h = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let overlap = w * h;
    let union = (a[2] - a[0]) * (a[3] - a[1]) + (b[2] - b[0]) * (b[3] - b[1]) - overlap;
    if union <= 0.0 { 0.0 } else { overlap / union }
}

fn coverage(boxes: &[Box4]) -> f32 {
    let mut covered = 0;
    for gy in 0..100 {
        for gx in 0..100 {
            let (x, y) = (gx as f32 * 10.0 + 5.0, gy as f32 * 10.0 + 5.0);
            if boxes
                .iter()
                .any(|b| x >= b[0] && x < b[2] && y >= b[1] && y < b[3])
            {
                covered += 1;
            }
        }
    }
    covered as f32 / 10_000.0
}

pub fn check_set(boxes: &[Option<Box4>]) -> Vec<Option<Reject>> {
    let mut verdicts: Vec<Option<Reject>> = vec![None; boxes.len()];
    for (later, later_box) in boxes.iter().enumerate() {
        let Some(later_box) = later_box else { continue };
        if boxes[..later]
            .iter()
            .flatten()
            .any(|earlier| iou(*earlier, *later_box) >= DUPLICATE_IOU)
        {
            verdicts[later] = Some(Reject::Duplicate);
        }
    }
    let kept: Vec<usize> = (0..boxes.len())
        .filter(|i| boxes[*i].is_some() && verdicts[*i].is_none())
        .collect();
    if kept.len() >= 2 {
        let all: Vec<Box4> = kept.iter().filter_map(|i| boxes[*i]).collect();
        if coverage(&all) >= TILING_SHARE {
            for index in kept {
                verdicts[index] = Some(Reject::TilesTheScreen);
            }
        }
    }
    verdicts
}

pub fn guard(findings: &mut [Finding]) -> u32 {
    let boxes: Vec<Option<Box4>> = findings.iter().map(|finding| finding.bbox).collect();
    let mut rejected = 0;
    for (finding, verdict) in findings.iter_mut().zip(check_set(&boxes)) {
        if let Some(reason) = verdict {
            log::info!(
                "analysis box {} ({}) dropped: {}",
                finding.n,
                finding.label,
                reason.why()
            );
            finding.bbox = None;
            rejected += 1;
        }
    }
    rejected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::report::Kind;

    #[test]
    fn a_plausible_box_passes_and_corners_are_put_right() {
        assert_eq!(
            usable_box([100.0, 200.0, 300.0, 260.0]),
            Ok([100.0, 200.0, 300.0, 260.0])
        );
        assert_eq!(
            usable_box([300.0, 260.0, 100.0, 200.0]),
            Ok([100.0, 200.0, 300.0, 260.0])
        );
    }

    #[test]
    fn malformed_numbers_are_rejected() {
        assert_eq!(
            usable_box([f32::NAN, 0.0, 10.0, 10.0]),
            Err(Reject::Malformed)
        );
        assert_eq!(
            usable_box([0.0, f32::INFINITY, 10.0, 10.0]),
            Err(Reject::Malformed)
        );
    }

    #[test]
    fn a_small_overrun_is_clamped_and_a_big_one_rejected() {
        assert_eq!(
            usable_box([-30.0, 100.0, 200.0, 200.0]),
            Ok([0.0, 100.0, 200.0, 200.0])
        );
        assert_eq!(
            usable_box([900.0, 900.0, 1030.0, 1040.0]),
            Ok([900.0, 900.0, 1000.0, 1000.0])
        );
        assert_eq!(
            usable_box([-50.0, 100.0, 200.0, 200.0]),
            Err(Reject::OutOfRange)
        );
        assert_eq!(
            usable_box([100.0, 100.0, 1200.0, 200.0]),
            Err(Reject::OutOfRange)
        );
        assert_eq!(
            usable_box([100.0, 100.0, 5000.0, 5000.0]),
            Err(Reject::OutOfRange)
        );
    }

    #[test]
    fn tiny_and_zero_sized_boxes_are_rejected() {
        assert_eq!(
            usable_box([100.0, 100.0, 107.0, 200.0]),
            Err(Reject::TooSmall)
        );
        assert_eq!(
            usable_box([100.0, 100.0, 108.0, 108.0]),
            Ok([100.0, 100.0, 108.0, 108.0])
        );
        assert_eq!(
            usable_box([100.0, 100.0, 100.0, 100.0]),
            Err(Reject::TooSmall)
        );
        assert_eq!(
            usable_box([-30.0, -30.0, 4.0, 4.0]),
            Err(Reject::TooSmall),
            "clamped to 4 by 4"
        );
    }

    #[test]
    fn a_box_over_sixty_percent_of_the_screen_is_rejected() {
        assert_eq!(
            usable_box([0.0, 0.0, 1000.0, 1000.0]),
            Err(Reject::TooLarge)
        );
        assert_eq!(
            usable_box([0.0, 0.0, 800.0, 800.0]),
            Err(Reject::TooLarge),
            "64 percent"
        );
        assert_eq!(
            usable_box([0.0, 0.0, 770.0, 770.0]),
            Ok([0.0, 0.0, 770.0, 770.0]),
            "59.3 percent"
        );
    }

    #[test]
    fn an_exact_duplicate_loses_its_box_but_the_first_keeps_its() {
        let a = Some([100.0, 100.0, 300.0, 200.0]);
        let verdicts = check_set(&[a, Some([400.0, 400.0, 500.0, 500.0]), a]);
        assert_eq!(verdicts, vec![None, None, Some(Reject::Duplicate)]);
    }

    #[test]
    fn three_boxes_that_tile_the_screen_are_all_rejected() {
        let thirds = [
            Some([0.0, 0.0, 333.0, 1000.0]),
            Some([333.0, 0.0, 666.0, 1000.0]),
            Some([666.0, 0.0, 1000.0, 1000.0]),
        ];
        assert_eq!(check_set(&thirds), vec![Some(Reject::TilesTheScreen); 3]);
        let rows = [
            Some([0.0, 0.0, 1000.0, 333.0]),
            Some([0.0, 333.0, 1000.0, 666.0]),
            Some([0.0, 666.0, 1000.0, 1000.0]),
        ];
        assert!(
            check_set(&rows)
                .iter()
                .all(|v| *v == Some(Reject::TilesTheScreen))
        );
        let grid = [
            Some([0.0, 0.0, 500.0, 500.0]),
            Some([500.0, 0.0, 1000.0, 500.0]),
            Some([0.0, 500.0, 500.0, 1000.0]),
            Some([500.0, 500.0, 1000.0, 1000.0]),
        ];
        assert!(
            check_set(&grid)
                .iter()
                .all(|v| *v == Some(Reject::TilesTheScreen))
        );
    }

    #[test]
    fn a_few_boxes_with_gaps_are_findings_not_a_tiling() {
        let boxes = [
            Some([50.0, 50.0, 400.0, 200.0]),
            Some([500.0, 300.0, 700.0, 350.0]),
            Some([100.0, 800.0, 300.0, 900.0]),
            None,
        ];
        assert_eq!(check_set(&boxes), vec![None, None, None, None]);
        assert_eq!(
            check_set(&[Some([0.0, 0.0, 500.0, 1000.0])]),
            vec![None],
            "one box cannot tile"
        );
        assert_eq!(check_set(&[]), Vec::<Option<Reject>>::new());
    }

    fn finding(n: u8, bbox: Option<Box4>) -> Finding {
        Finding {
            group: crate::analysis::report::Group::default(),
            n,
            label: format!("F{n}"),
            kind: Kind::Region,
            bbox,
            description: String::new(),
            evidence: String::new(),
            point: None,
            pose: None,
        }
    }

    #[test]
    fn the_guard_removes_only_the_bad_boxes_and_counts_them() {
        let mut findings = vec![
            finding(1, Some([0.0, 0.0, 333.0, 1000.0])),
            finding(2, Some([333.0, 0.0, 666.0, 1000.0])),
            finding(3, Some([666.0, 0.0, 1000.0, 1000.0])),
            finding(4, Some([100.0, 100.0, 200.0, 150.0])),
        ];
        let rejected = guard(&mut findings);
        assert_eq!(
            rejected, 4,
            "all four: the union of all four still covers the screen"
        );
        assert!(findings.iter().all(|f| f.bbox.is_none()));
        let mut mixed = vec![
            finding(1, Some([100.0, 100.0, 300.0, 200.0])),
            finding(2, None),
            finding(3, Some([500.0, 500.0, 600.0, 560.0])),
        ];
        assert_eq!(guard(&mut mixed), 0);
        assert!(mixed[0].bbox.is_some() && mixed[2].bbox.is_some());
        assert_eq!(mixed[1].label, "F2", "findings keep their text");
    }
}
