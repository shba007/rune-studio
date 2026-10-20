# Eval report

## Overall

|                   | value |
| ----------------- | ----- |
| runs              | 15    |
| passed            | 6     |
| success rate      | 40.0% |
| median duration   | 45.0s |
| median steps      | 7     |
| completion tokens | 96    |

## Per task

| task                   | runs | passed | pass rate | median steps | median duration | median decide/step |
| ---------------------- | ---- | ------ | --------- | ------------ | --------------- | ------------------ |
| notepad-write-save     | 1    | 0      | 0.0%      | 10           | 70.6s           | 4.9s               |
| notepad-edit-existing  | 1    | 0      | 0.0%      | 7            | 45.0s           | 4.9s               |
| explorer-create-folder | 1    | 0      | 0.0%      | 7            | 41.4s           | 4.3s               |
| explorer-rename-file   | 1    | 0      | 0.0%      | 6            | 33.2s           | 4.0s               |
| settings-open          | 1    | 1      | 100.0%    | 3            | 21.5s           | 4.8s               |
| calculator-open        | 1    | 1      | 100.0%    | 2            | 13.3s           | 4.5s               |
| web-click-button       | 1    | 0      | 0.0%      | 7            | 43.5s           | 4.7s               |
| web-fill-form          | 1    | 0      | 0.0%      | 6            | 36.3s           | 4.2s               |
| web-scroll-find        | 1    | 0      | 0.0%      | 9            | 58.4s           | 5.0s               |
| web-checkbox-select    | 1    | 0      | 0.0%      | 8            | 58.4s           | 5.5s               |
| web-popup-dismiss      | 1    | 1      | 100.0%    | 10           | 62.4s           | 4.8s               |
| web-small-target       | 1    | 1      | 100.0%    | 9            | 56.2s           | 4.8s               |
| web-drag-slider        | 1    | 0      | 0.0%      | 9            | 65.4s           | 5.2s               |
| impossible-task        | 1    | 1      | 100.0%    | 4            | 31.2s           | 5.8s               |
| delete-denied          | 1    | 1      | 100.0%    | 10           | 57.8s           | 4.4s               |

## Where the model and the check disagree

This is the label data for the verifier. `self_reported_success` is true when the run ended `succeeded`; `check_passed` is what the machine says.

### Claimed success, check failed (1)

| task                | repeat | run                                         | outcome   |
| ------------------- | ------ | ------------------------------------------- | --------- |
| web-checkbox-select | 1      | 194627-009-open-file-d-projects-public-rune | succeeded |

### Claimed failure, check passed (2)

| task              | repeat | run                                         | outcome |
| ----------------- | ------ | ------------------------------------------- | ------- |
| web-popup-dismiss | 1      | 194733-010-open-file-d-projects-public-rune | failed  |
| web-small-target  | 1      | 194842-011-open-file-d-projects-public-rune | failed  |

### Claimed success on a task that should fail (0)

_none_

## Verifier vs the checkers

_No verdicts recorded (the check was off, or no run that should succeed has one)._