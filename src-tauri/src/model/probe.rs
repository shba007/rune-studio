use std::time::Duration;

use crate::model::{
    capabilities::TriState,
    coords,
    profile::{ModelProfile, PointingConvention, ThinkingStyle, ThinkingSwitch},
};

/// An observed point response for a known target box on a rendered scene.
#[derive(Debug, Clone, PartialEq)]
pub struct PointObservation {
    pub raw_point: (f64, f64),
    pub image_dims: (u32, u32),
    pub target_norm1000: [f64; 4], // [l, t, r, b] in Studio's 0..1000 space
}

/// Evaluates which candidate pointing convention (norm1000, pixels, norm1)
/// best explains the observed points against known targets.
pub fn score_pointing_conventions(observations: &[PointObservation]) -> PointingConvention {
    if observations.is_empty() {
        return PointingConvention::Unknown;
    }

    let candidates = [
        PointingConvention::Norm1000,
        PointingConvention::Pixels,
        PointingConvention::Norm1,
    ];

    let mut best_convention = PointingConvention::Unknown;
    let mut best_hits = 0;

    for &candidate in &candidates {
        let mut hits = 0;
        for obs in observations {
            let (nx, ny) = coords::adapt_point_to_norm1000(
                candidate,
                obs.raw_point.0,
                obs.raw_point.1,
                obs.image_dims.0,
                obs.image_dims.1,
            );
            let [l, t, r, b] = obs.target_norm1000;
            // Near margin of 15 units in 0..1000 coordinate space
            let margin = 15.0;
            if nx >= l - margin && nx <= r + margin && ny >= t - margin && ny <= b + margin {
                hits += 1;
            }
        }
        if hits > best_hits {
            best_hits = hits;
            best_convention = candidate;
        }
    }

    if best_hits == 0 {
        PointingConvention::Unknown
    } else {
        best_convention
    }
}

/// An observation of a thinking probe run with a given request style.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinkingProbeObservation {
    pub style: ThinkingStyle,
    pub duration: Duration,
    pub reasoning_chars: usize,
}

/// Evaluates which request styles successfully disable or switch thinking,
/// comparing duration and presence of reasoning against a baseline with thinking on.
pub fn score_thinking_styles(
    baseline_on_duration: Duration,
    baseline_on_reasoning_chars: usize,
    probes: &[ThinkingProbeObservation],
) -> ThinkingSwitch {
    let mut working_styles = Vec::new();

    for probe in probes {
        let reasoning_suppressed = if baseline_on_reasoning_chars > 0 {
            probe.reasoning_chars == 0 || probe.reasoning_chars * 5 < baseline_on_reasoning_chars
        } else {
            probe.reasoning_chars == 0
        };

        // Noticeable duration reduction (at least 20% faster than baseline with thinking on)
        let faster = probe.duration.as_millis() * 10 <= baseline_on_duration.as_millis() * 8;

        if reasoning_suppressed && (probe.reasoning_chars == 0 || faster) {
            working_styles.push(probe.style);
        }
    }

    if working_styles.is_empty() {
        ThinkingSwitch::None
    } else {
        ThinkingSwitch::Styles(working_styles)
    }
}

/// An observation of a forced tool-call attempt.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallObservation {
    pub called: bool,
    pub valid_arguments: bool,
}

/// Counts valid vs malformed tool call arguments over N tries.
pub fn score_tool_choice(observations: &[ToolCallObservation]) -> TriState {
    if observations.is_empty() {
        return TriState::Unknown;
    }
    let total = observations.len();
    let valid_count = observations
        .iter()
        .filter(|o| o.called && o.valid_arguments)
        .count();
    let malformed_count = observations
        .iter()
        .filter(|o| o.called && !o.valid_arguments)
        .count();

    if valid_count == total || (valid_count * 2 >= total && malformed_count == 0) {
        TriState::Yes
    } else if valid_count == 0 || malformed_count > total / 2 {
        TriState::No
    } else {
        TriState::Unknown
    }
}

/// An observation of an image resolution probe trial.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageResolutionObservation {
    pub max_edge: u32,
    pub success: bool,
    pub latency: Duration,
}

/// Suggests a safe maximum image edge based on tested resolutions.
pub fn score_max_image_edge(observations: &[ImageResolutionObservation]) -> u32 {
    let mut highest_successful = 1280;
    for obs in observations {
        if obs.success && obs.max_edge > highest_successful {
            highest_successful = obs.max_edge;
        }
    }
    highest_successful
}

/// Constructs a suggested ModelProfile from the scored probe results.
pub fn build_suggested_profile(
    model_id: &str,
    thinking: ThinkingSwitch,
    pointing: PointingConvention,
    tool_choice: TriState,
    max_image_edge: u32,
) -> ModelProfile {
    let mut profile = ModelProfile::conservative(model_id);
    profile.thinking = thinking;
    profile.pointing = pointing;
    profile.tool_choice_required = tool_choice;
    profile.images.max_edge_px = max_image_edge;
    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_pointing_conventions_norm1000() {
        // Targets in norm1000 space: [100, 100, 200, 200]
        let obs = vec![
            PointObservation {
                raw_point: (150.0, 150.0), // Norm1000 coordinates
                image_dims: (1280, 720),
                target_norm1000: [100.0, 100.0, 200.0, 200.0],
            },
            PointObservation {
                raw_point: (500.0, 500.0), // Norm1000 coordinates
                image_dims: (1280, 720),
                target_norm1000: [480.0, 480.0, 520.0, 520.0],
            },
        ];
        assert_eq!(
            score_pointing_conventions(&obs),
            PointingConvention::Norm1000
        );
    }

    #[test]
    fn test_score_pointing_conventions_pixels() {
        // Target at [500, 500, 600, 600] in norm1000 -> in 1280x720 pixels: x=640..768, y=360..432
        // Centre is roughly (704, 396)
        let obs = vec![
            PointObservation {
                raw_point: (704.0, 396.0),
                image_dims: (1280, 720),
                target_norm1000: [500.0, 500.0, 600.0, 600.0],
            },
            PointObservation {
                raw_point: (320.0, 180.0), // ~250 norm
                image_dims: (1280, 720),
                target_norm1000: [200.0, 200.0, 300.0, 300.0],
            },
        ];
        assert_eq!(score_pointing_conventions(&obs), PointingConvention::Pixels);
    }

    #[test]
    fn test_score_pointing_conventions_norm1() {
        // Coordinates in 0..1 range
        let obs = vec![
            PointObservation {
                raw_point: (0.55, 0.55),
                image_dims: (1280, 720),
                target_norm1000: [500.0, 500.0, 600.0, 600.0],
            },
            PointObservation {
                raw_point: (0.25, 0.25),
                image_dims: (1280, 720),
                target_norm1000: [200.0, 200.0, 300.0, 300.0],
            },
        ];
        assert_eq!(score_pointing_conventions(&obs), PointingConvention::Norm1);
    }

    #[test]
    fn test_score_pointing_conventions_unknown_on_misses() {
        let obs = vec![PointObservation {
            raw_point: (9999.0, 9999.0),
            image_dims: (1280, 720),
            target_norm1000: [100.0, 100.0, 200.0, 200.0],
        }];
        assert_eq!(
            score_pointing_conventions(&obs),
            PointingConvention::Unknown
        );
        assert_eq!(score_pointing_conventions(&[]), PointingConvention::Unknown);
    }

    #[test]
    fn test_score_thinking_styles_identifies_working_styles() {
        let baseline_duration = Duration::from_secs(18);
        let baseline_reasoning = 1200;

        let probes = vec![
            ThinkingProbeObservation {
                style: ThinkingStyle::ChatTemplateKwargs,
                duration: Duration::from_millis(4500),
                reasoning_chars: 0,
            },
            ThinkingProbeObservation {
                style: ThinkingStyle::ThinkingType,
                duration: Duration::from_secs(17),
                reasoning_chars: 1100,
            },
        ];

        let result = score_thinking_styles(baseline_duration, baseline_reasoning, &probes);
        assert_eq!(
            result,
            ThinkingSwitch::Styles(vec![ThinkingStyle::ChatTemplateKwargs])
        );
    }

    #[test]
    fn test_score_thinking_styles_returns_none_when_unswitchable() {
        let baseline_duration = Duration::from_secs(10);
        let baseline_reasoning = 800;

        let probes = vec![ThinkingProbeObservation {
            style: ThinkingStyle::ThinkingType,
            duration: Duration::from_secs(10),
            reasoning_chars: 850,
        }];

        let result = score_thinking_styles(baseline_duration, baseline_reasoning, &probes);
        assert_eq!(result, ThinkingSwitch::None);
    }

    #[test]
    fn test_score_tool_choice_distinguishes_reliability() {
        let all_valid = vec![
            ToolCallObservation {
                called: true,
                valid_arguments: true,
            },
            ToolCallObservation {
                called: true,
                valid_arguments: true,
            },
        ];
        assert_eq!(score_tool_choice(&all_valid), TriState::Yes);

        let malformed = vec![
            ToolCallObservation {
                called: true,
                valid_arguments: false,
            },
            ToolCallObservation {
                called: true,
                valid_arguments: false,
            },
        ];
        assert_eq!(score_tool_choice(&malformed), TriState::No);

        assert_eq!(score_tool_choice(&[]), TriState::Unknown);
    }

    #[test]
    fn test_score_max_image_edge() {
        let probes = vec![
            ImageResolutionObservation {
                max_edge: 1280,
                success: true,
                latency: Duration::from_millis(500),
            },
            ImageResolutionObservation {
                max_edge: 1920,
                success: true,
                latency: Duration::from_millis(900),
            },
        ];
        assert_eq!(score_max_image_edge(&probes), 1920);

        let failed_high = vec![
            ImageResolutionObservation {
                max_edge: 1280,
                success: true,
                latency: Duration::from_millis(500),
            },
            ImageResolutionObservation {
                max_edge: 1920,
                success: false,
                latency: Duration::from_secs(30),
            },
        ];
        assert_eq!(score_max_image_edge(&failed_high), 1280);
    }

    #[test]
    fn test_build_suggested_profile_structure() {
        let profile = build_suggested_profile(
            "custom-calibrated-model",
            ThinkingSwitch::Styles(vec![ThinkingStyle::ChatTemplateKwargs]),
            PointingConvention::Norm1000,
            TriState::Yes,
            1920,
        );

        assert_eq!(profile.id, "custom-calibrated-model");
        assert_eq!(
            profile.thinking,
            ThinkingSwitch::Styles(vec![ThinkingStyle::ChatTemplateKwargs])
        );
        assert_eq!(profile.pointing, PointingConvention::Norm1000);
        assert_eq!(profile.tool_choice_required, TriState::Yes);
        assert_eq!(profile.images.max_edge_px, 1920);

        // Valid JSON serialization
        let json_str = serde_json::to_string_pretty(&profile).unwrap();
        assert!(json_str.contains("\"id\": \"custom-calibrated-model\""));
        assert!(json_str.contains("\"chat_template_kwargs\""));
        assert!(json_str.contains("\"pointing\": \"norm1000\""));
    }
}
