#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub path: String,
    pub hold_ms: u64,
}

pub const MIN_HOLD_MS: u64 = 200;
pub const LAST_HOLD_MS: u64 = 1_000;

pub fn schedule(frames: &[(String, u64)], end_ms: Option<u64>) -> Vec<Shown> {
    let mut sorted: Vec<(String, u64)> = frames.to_vec();
    sorted.sort_by_key(|(_, at)| *at);
    let mut unique: Vec<(String, u64)> = Vec::new();
    for (path, at) in sorted {
        match unique.last_mut() {
            Some(last) if last.1 == at => *last = (path, at),
            _ => unique.push((path, at)),
        }
    }
    if let Some(first) = unique.first_mut() {
        first.1 = 0;
    }
    let mut shown = Vec::with_capacity(unique.len());
    for (index, (path, at)) in unique.iter().enumerate() {
        let hold = match unique.get(index + 1) {
            Some((_, next)) => next.saturating_sub(*at),
            None => end_ms.map_or(LAST_HOLD_MS, |end| end.saturating_sub(*at).max(MIN_HOLD_MS)),
        };
        shown.push(Shown {
            path: path.clone(),
            hold_ms: hold.max(MIN_HOLD_MS),
        });
    }
    shown
}

pub fn total_ms(shown: &[Shown]) -> u64 {
    shown.iter().map(|frame| frame.hold_ms).sum()
}

pub fn video_size(width: u32, height: u32, max_width: u32) -> (u32, u32) {
    let (mut w, mut h) = (width.max(2), height.max(2));
    if w > max_width {
        h = (u64::from(h) * u64::from(max_width) / u64::from(w)) as u32;
        w = max_width;
    }
    (w & !1, (h & !1).max(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(path: &str, at: u64) -> (String, u64) {
        (path.to_string(), at)
    }

    #[test]
    fn each_frame_is_held_until_the_next_and_the_last_until_the_end() {
        let shown = schedule(
            &[frame("a", 0), frame("b", 7_000), frame("c", 15_000)],
            Some(55_000),
        );
        assert_eq!(
            shown.iter().map(|f| f.hold_ms).collect::<Vec<_>>(),
            [7_000, 8_000, 40_000]
        );
        assert_eq!(
            total_ms(&shown),
            55_000,
            "the video is as long as the session"
        );
    }

    #[test]
    fn the_first_frame_starts_the_video_even_if_it_was_taken_later() {
        let shown = schedule(&[frame("a", 900), frame("b", 5_000)], Some(10_000));
        assert_eq!(shown[0].hold_ms, 5_000, "a is on screen from time 0 to b");
        assert_eq!(total_ms(&shown), 10_000);
    }

    #[test]
    fn frames_out_of_order_or_at_the_same_moment_are_tidied() {
        let shown = schedule(
            &[
                frame("c", 9_000),
                frame("a", 0),
                frame("b1", 4_000),
                frame("b2", 4_000),
            ],
            Some(12_000),
        );
        assert_eq!(
            shown.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
            ["a", "b2", "c"]
        );
        assert_eq!(total_ms(&shown), 12_000);
    }

    #[test]
    fn a_session_shorter_than_a_second_and_a_single_frame_still_make_a_video() {
        let one = schedule(&[frame("a", 0)], Some(400));
        assert_eq!(
            one,
            vec![Shown {
                path: "a".into(),
                hold_ms: 400
            }]
        );
        let unknown_end = schedule(&[frame("a", 0)], None);
        assert_eq!(unknown_end[0].hold_ms, LAST_HOLD_MS);
        let blink = schedule(&[frame("a", 0), frame("b", 30)], Some(100));
        assert!(
            blink.iter().all(|f| f.hold_ms >= MIN_HOLD_MS),
            "no unreadable flash: {blink:?}"
        );
    }

    #[test]
    fn no_frames_means_no_video() {
        assert!(schedule(&[], Some(5_000)).is_empty());
        assert_eq!(total_ms(&[]), 0);
    }

    #[test]
    fn the_size_is_even_capped_and_keeps_the_aspect() {
        assert_eq!(video_size(1920, 1080, 1280), (1280, 720));
        assert_eq!(
            video_size(1366, 768, 1280),
            (1280, 718),
            "719.6 rounded down to even"
        );
        assert_eq!(
            video_size(1281, 721, 2000),
            (1280, 720),
            "odd sizes are made even"
        );
        assert_eq!(video_size(800, 600, 1280), (800, 600), "never enlarged");
        assert_eq!(video_size(3840, 1600, 1280), (1280, 532));
        assert_eq!(video_size(1, 1, 1280), (2, 2));
    }
}
