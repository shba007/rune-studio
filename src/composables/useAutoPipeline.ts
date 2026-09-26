import { ref } from 'vue'
import { useMediaStore } from '@/stores/mediaStore'
import { useMediaProfiler } from '@/composables/useMediaProfiler'
import type { MediaItem, IdentityCluster } from '@/types/media'
import type { MediaArtifact } from '@/types/artifact'

export function useAutoPipeline() {
  const mediaStore = useMediaStore()
  const { profileAsset, arbitratePair } = useMediaProfiler()
  const isCancelled = ref(false)

  function cancelPipeline() {
    isCancelled.value = true
    mediaStore.pipelineState = 'idle'
    mediaStore.pipelineProgress.currentItem = null
  }

  /**
   * Stage 1: Quality-Gated Multi-Exemplar Star Clustering
   * Complexity: O(N * K) where K is number of unique identities (typically 2 to 5)
   */
  async function runQualityGatedClustering(
    sortedItems: MediaItem[],
    artifactsMap: Map<string, MediaArtifact>,
    threshold: number,
  ): Promise<IdentityCluster[]> {
    const clusters: IdentityCluster[] = []
    const total = sortedItems.length

    for (let i = 0; i < total; i++) {
      if (isCancelled.value) return []

      const candidate = sortedItems[i]!
      const candidateArt = artifactsMap.get(candidate.id)
      mediaStore.pipelineProgress.current = i + 1
      mediaStore.pipelineProgress.currentItem = candidate
      mediaStore.pipelineProgress.currentStage = `Arbitrating ${i + 1}/${total}: ${candidate.name}`

      let matchedCluster: IdentityCluster | null = null
      let highestMatchConfidence = 0

      // Probe existing clusters against their Exemplar Pool
      for (const cluster of clusters) {
        if (isCancelled.value) return []

        for (const exemplar of cluster.exemplars) {
          const verdict = await arbitratePair(candidate, exemplar, mediaStore.selectedIntentKey)

          if (verdict.isMatch && verdict.confidence >= threshold) {
            matchedCluster = cluster
            highestMatchConfidence = Math.max(highestMatchConfidence, verdict.confidence)
            break // Early Exit: Match confirmed for this cluster
          }
        }

        if (matchedCluster) {
          break // Early Exit: Found corresponding cluster, stop probing remaining clusters
        }
      }

      if (matchedCluster) {
        matchedCluster.items.push(candidate)
        matchedCluster.confidenceAverage =
          Math.round(((matchedCluster.confidenceAverage + highestMatchConfidence) / 2) * 100) / 100

        // Exemplar Promotion: If candidate has high fidelity and provides a complementary angle, add to exemplar pool
        if (
          matchedCluster.exemplars.length < 3 &&
          candidateArt &&
          candidateArt.fidelityScore >= 7.0
        ) {
          const currentAngles = new Set(
            matchedCluster.exemplars.map((ex) => artifactsMap.get(ex.id)?.assessment.viewAngle),
          )
          if (!currentAngles.has(candidateArt.assessment.viewAngle)) {
            matchedCluster.exemplars.push(candidate)
          }
        }
      } else {
        // Unmatched: Spawn a new cluster with this candidate as the Primary Seed & Initial Exemplar
        const clusterIdx = clusters.length + 1
        const defaultName = `person-${clusterIdx}`
        clusters.push({
          id: `cluster-${Date.now()}-${clusterIdx}`,
          suggestedName: defaultName,
          customName: defaultName,
          primaryItem: candidate,
          exemplars: [candidate],
          items: [candidate],
          confidenceAverage: 1.0,
        })
      }
    }

    // Stage 2: Fast Cluster Reconciliation Pass (O(K^2) with K <= 5)
    // Merges boundary clusters if their secondary exemplars confirm same identity
    if (clusters.length > 1) {
      mediaStore.pipelineProgress.currentStage = 'Reconciling cluster boundaries...'
      let merged = true
      while (merged && clusters.length > 1) {
        merged = false
        for (let a = 0; a < clusters.length; a++) {
          for (let b = a + 1; b < clusters.length; b++) {
            const clusterA = clusters[a]!
            const clusterB = clusters[b]!

            // Arbitrate primary seeds of Cluster A and Cluster B
            const verdict = await arbitratePair(
              clusterA.primaryItem,
              clusterB.primaryItem,
              mediaStore.selectedIntentKey,
            )

            if (verdict.isMatch && verdict.confidence >= threshold) {
              // Merge Cluster B into Cluster A
              clusterA.items.push(...clusterB.items)
              for (const exB of clusterB.exemplars) {
                if (clusterA.exemplars.length < 3) {
                  clusterA.exemplars.push(exB)
                }
              }
              clusters.splice(b, 1)
              merged = true
              break
            }
          }
          if (merged) break
        }
      }
    }

    return clusters
  }

  async function startAutoPipeline() {
    if (mediaStore.mediaItems.length === 0) return

    isCancelled.value = false
    mediaStore.pipelineState = 'profiling'
    const total = mediaStore.mediaItems.length
    const artifactsMap = new Map<string, MediaArtifact>()

    try {
      // 1. Single-Image Profiling & Fidelity Score Calculation
      for (let i = 0; i < total; i++) {
        if (isCancelled.value) return

        const item = mediaStore.mediaItems[i]!
        mediaStore.pipelineProgress = {
          current: i,
          total,
          currentItem: item,
          currentStage: 'Profiling Visual Quality & Viewpoints...',
        }

        const artifact = await profileAsset(item)
        if (artifact) {
          artifactsMap.set(item.id, artifact)
        }
      }

      if (isCancelled.value) return

      // 2. Quality-Gated Sorting: Assets sorted by Fidelity Score DESCENDING
      // High-resolution, unoccluded images are tested first as optimal cluster anchors
      const sortedCandidates = [...mediaStore.mediaItems].sort((a, b) => {
        const scoreA = artifactsMap.get(a.id)?.fidelityScore ?? 0
        const scoreB = artifactsMap.get(b.id)?.fidelityScore ?? 0
        return scoreB - scoreA
      })

      mediaStore.pipelineState = 'clustering'
      const clusters = await runQualityGatedClustering(
        sortedCandidates,
        artifactsMap,
        mediaStore.confidenceThreshold,
      )

      if (isCancelled.value) return

      mediaStore.identityClusters = clusters
      mediaStore.pipelineState = 'review'
      mediaStore.isIdentityReviewOpen = true
    } catch (err) {
      console.error('[useAutoPipeline] Pipeline error:', err)
      mediaStore.pipelineState = 'idle'
    } finally {
      mediaStore.pipelineProgress.currentItem = null
    }
  }

  function stageAllClusterActions() {
    for (const cluster of mediaStore.identityClusters) {
      if (cluster.items.length === 0) continue

      const safeFolderName =
        cluster.customName.trim().replace(/[^a-zA-Z0-9_-]/g, '_') || cluster.suggestedName
      const targetDir = `${mediaStore.directoryPath}/_identity_${safeFolderName}`

      for (const item of cluster.items) {
        mediaStore.stageAction({
          type: 'MOVE_SAME_DIR',
          sourceItem: item,
          matchedWith: cluster.primaryItem,
          targetDirectory: targetDir,
          confidence: cluster.confidenceAverage,
          reason: `Grouped under identity "${cluster.customName}" via multi-exemplar arbitration`,
        })
      }
    }

    mediaStore.isIdentityReviewOpen = false
    mediaStore.pipelineState = 'completed'
    mediaStore.isReviewModalOpen = true
  }

  return {
    isCancelled,
    startAutoPipeline,
    cancelPipeline,
    stageAllClusterActions,
  }
}