import type { AssetAssessment } from '@/types/artifact'

/**
 * Calculates a continuous forensic fidelity score (0.0 to 10.0)
 * used for Quality-Gated Seed Sorting so occluded/low-res shots are never chosen as anchors.
 */
export function calculateFidelityScore(assessment: AssetAssessment): number {
  if (!assessment.subjectDetected) return 0.0

  let score = 0.0
  const q = assessment.observationQuality

  // 1. Resolution (max 3.0 pts)
  if (q.resolutionClass === 'high') score += 3.0
  else if (q.resolutionClass === 'medium') score += 2.0
  else score += 1.0

  // 2. Illumination (max 2.5 pts)
  if (q.illumination === 'uniform') score += 2.5
  else if (q.illumination === 'side_shadow') score += 1.8
  else if (q.illumination === 'backlit') score += 1.2
  else score += 0.8

  // 3. Occlusion (max 2.5 pts)
  if (q.occlusionLevel === 'none') score += 2.5
  else if (q.occlusionLevel === 'partial_under_25') score += 1.5
  else if (q.occlusionLevel === 'heavy_under_60') score += 0.5
  else score += 0.0

  // 4. Motion Blur (max 1.0 pt)
  if (q.motionBlur === 'none') score += 1.0
  else if (q.motionBlur === 'slight') score += 0.5
  else score += 0.1

  // 5. Angle bonus (max 1.0 pt) - Frontal & 3/4 views are superior anchor candidates
  if (assessment.viewAngle === 'frontal') score += 1.0
  else if (assessment.viewAngle === 'three_quarter') score += 0.8
  else if (assessment.viewAngle === 'profile') score += 0.4
  else score += 0.1

  return Math.min(10.0, Math.max(0.0, Math.round(score * 10) / 10))
}

/**
 * Generates the target-conditioned prompt for direct dual-image VLM arbitration.
 */
export function buildArbitrationPrompt(intentKey: string): {
  systemPrompt: string
  userInstruction: string
} {
  switch (intentKey) {
    case 'same_person':
      return {
        systemPrompt: `You are an expert forensic facial examiner and visual biometrics arbitrator.
You are provided two images (Image A and Image B).
Your task is to determine whether both images depict the EXACT SAME INDIVIDUAL.
MANDATORY RULES:
1. Examine invariant anatomical landmarks: craniofacial bone structure, interpupillary distance ratio, ear shape/lobule attachment, nasal root and bridge profile, philtrum definition, and chin contour.
2. STRICTLY IGNORE: Clothing, jewellery, background, room setting, transient hairstyle changes (e.g. ponytail vs hair down), and accessories (such as glasses or hats).
3. If one image has partial occlusion (e.g. sunglasses), evaluate identity ONLY using the remaining visible invariant structures (jawline, ears, nose tip, mouth). Do NOT reject merely due to an occluded area.
4. Output valid JSON strictly conforming to the requested schema.`,
        userInstruction: `Compare Image A and Image B. Do they depict the exact same person?`,
      }

    case 'same_product':
      return {
        systemPrompt: `You are a forensic product verification engine.
Compare Image A and Image B to determine whether they depict the exact same physical product model.
Analyze silhouette geometry, edge contours, seams, logos, hardware, and materials.
Ignore the model/wearer, room background, and differences in lighting or camera angle.`,
        userInstruction: `Compare Image A and Image B. Do they depict the exact same product model?`,
      }

    case 'duplicates':
      return {
        systemPrompt: `You are a photographic deduplication engine.
Compare Image A and Image B to determine if they are duplicate captures, near-duplicates, or identical burst shots.
Analyze framing, camera angle, subject pose, and composition.`,
        userInstruction: `Are Image A and Image B identical captures or near-duplicates?`,
      }

    case 'same_category':
    default:
      return {
        systemPrompt: `You are a visual scene and context categorization engine.
Determine whether Image A and Image B share the same thematic category, scene setting, or ambiance.`,
        userInstruction: `Do Image A and Image B belong to the exact same thematic category or scene setting?`,
      }
  }
}