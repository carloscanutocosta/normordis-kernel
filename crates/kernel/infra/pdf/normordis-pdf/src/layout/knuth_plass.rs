//! Knuth-Plass optimal line-breaking algorithm (1981).
//!
//! Minimises total paragraph "badness" by choosing break-points via dynamic
//! programming rather than the greedy first-fit approach.  Enabled at
//! compile-time by the `optimal_wrap` feature flag.

/// A single word-box: a token that cannot be broken internally.
#[derive(Debug, Clone)]
pub struct WordBox {
    /// Natural rendered width in mm.
    pub width: f64,
}

/// Infinity sentinel — larger than any realistic demerits (cubic formula peaks ~10^14).
const DEMERITS_INF: f64 = 1e20;
const DEMERITS_CONSECUTIVE: f64 = 50.0;
const DEMERITS_WIDOW: f64 = 100.0;

/// Knuth-Plass optimizer for a single paragraph.
pub struct KnuthPlassOptimizer {
    /// Natural width of one inter-word space in mm.
    pub space_width: f64,
    /// Maximum extra stretch per space (fraction of `space_width`).
    /// A value of 0.5 means each space can grow up to 1.5× its natural width.
    pub space_stretch: f64,
    /// Maximum shrink per space (fraction of `space_width`).
    /// A value of 0.333 means each space can shrink to 0.667× its natural width.
    pub space_shrink: f64,
    /// Available line width in mm.
    pub line_width: f64,
}

impl KnuthPlassOptimizer {
    /// Creates an optimizer with typical typographic parameters.
    pub fn new(line_width: f64, space_width: f64) -> Self {
        Self {
            space_width,
            space_stretch: 0.5,
            space_shrink: 0.333,
            line_width,
        }
    }

    /// Returns the break-point indices for `boxes`.
    ///
    /// Each returned index `k` means the line ends after `boxes[k]` (inclusive,
    /// 0-based).  The caller reconstructs lines as `[prev_break+1 ..= k]`.
    ///
    /// Returns an empty `Vec` when `boxes` is empty; a single entry equal to
    /// `boxes.len() - 1` when everything fits on one line.
    pub fn optimize(&self, boxes: &[WordBox]) -> Vec<usize> {
        let n = boxes.len();
        if n == 0 {
            return vec![];
        }

        // dp[i] = minimum accumulated demerits to have broken all words [0..i].
        // choice[i] = the start index of the last line ending at word i-1.
        let mut dp = vec![DEMERITS_INF; n + 1];
        let mut choice = vec![0usize; n + 1];
        // previous line's adjustment ratio (for consecutive-demerits penalty)
        let mut prev_ratio = vec![0.0f64; n + 1];

        dp[0] = 0.0;

        for j in 1..=n {
            let is_last_line = j == n;

            // Try all line start positions i for the line [i..j], from
            // shortest line (i = j-1) to longest (i = 0).  We break as soon
            // as the line becomes too tight (ratio < -1) because adding even
            // more words can only make it tighter.
            for i in (0..j).rev() {
                if dp[i] >= DEMERITS_INF {
                    continue;
                }
                let words_in_line = j - i;
                let spaces = if words_in_line > 1 {
                    (words_in_line - 1) as f64
                } else {
                    0.0
                };
                let natural_words: f64 = boxes[i..j].iter().map(|b| b.width).sum();
                let natural = natural_words + spaces * self.space_width;

                // Last lines of a paragraph are always typeset flush-left:
                // their adjustment ratio is effectively 0 (no stretching needed).
                let ratio = if is_last_line {
                    if natural > self.line_width {
                        // Last line is too tight — still infeasible.
                        let r = self.adjustment_ratio(natural, spaces.max(1.0));
                        if r < -1.0 {
                            break;
                        }
                        r
                    } else {
                        0.0
                    }
                } else {
                    // Use effective_spaces ≥ 1 so single-word intermediate lines
                    // get a proper (large positive) ratio instead of 0.
                    let r = self.adjustment_ratio(natural, spaces.max(1.0));
                    if r < -1.0 {
                        // Too tight — no point trying to start even earlier on this line.
                        break;
                    }
                    r
                };

                let d = self.demerits(ratio);
                if d >= DEMERITS_INF {
                    continue;
                }

                // Consecutive loose/tight-line penalty.
                let consec = if i > 0
                    && ((ratio > 1.0 && prev_ratio[i] > 1.0)
                        || (ratio < 0.0 && prev_ratio[i] < 0.0))
                {
                    DEMERITS_CONSECUTIVE
                } else {
                    0.0
                };

                // Widow penalty: single-word last line.
                let widow = if is_last_line && words_in_line == 1 {
                    DEMERITS_WIDOW
                } else {
                    0.0
                };

                let total = dp[i] + d + consec + widow;
                if total < dp[j] {
                    dp[j] = total;
                    choice[j] = i;
                    prev_ratio[j] = ratio;
                }
            }
        }

        // Back-track to recover break-points.
        let mut breaks: Vec<usize> = Vec::new();
        let mut pos = n;
        loop {
            let start = choice[pos];
            breaks.push(pos - 1); // last word index of this line (0-based)
            if start == 0 {
                break;
            }
            pos = start;
        }
        breaks.reverse();
        breaks
    }

    fn adjustment_ratio(&self, natural: f64, spaces: f64) -> f64 {
        let diff = self.line_width - natural;
        if diff.abs() < 1e-9 || spaces == 0.0 {
            return 0.0;
        }
        if diff > 0.0 {
            // Needs to stretch.
            let max_stretch = spaces * self.space_width * self.space_stretch;
            if max_stretch <= 0.0 {
                return f64::INFINITY;
            }
            diff / max_stretch
        } else {
            // Needs to shrink.
            let max_shrink = spaces * self.space_width * self.space_shrink;
            if max_shrink <= 0.0 {
                return -f64::INFINITY;
            }
            diff / max_shrink
        }
    }

    fn demerits(&self, ratio: f64) -> f64 {
        if ratio < -1.0 {
            return DEMERITS_INF;
        }
        let bad = 1.0 + 100.0 * ratio.abs().powi(3);
        bad * bad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        let opt = KnuthPlassOptimizer::new(160.0, 2.5);
        assert!(opt.optimize(&[]).is_empty());
    }

    #[test]
    fn single_word_returns_one_break() {
        let opt = KnuthPlassOptimizer::new(160.0, 2.5);
        let boxes = vec![WordBox { width: 20.0 }];
        let breaks = opt.optimize(&boxes);
        assert_eq!(breaks, vec![0]);
    }

    #[test]
    fn all_words_fit_on_one_line() {
        let opt = KnuthPlassOptimizer::new(160.0, 2.5);
        let boxes: Vec<WordBox> = (0..5).map(|_| WordBox { width: 20.0 }).collect();
        let breaks = opt.optimize(&boxes);
        // 5 words × 20mm + 4 spaces × 2.5mm = 110mm ≤ 160mm
        assert_eq!(breaks, vec![4]);
    }

    #[test]
    fn two_lines_when_words_too_wide() {
        let opt = KnuthPlassOptimizer::new(60.0, 2.5);
        // 4 words × 20mm + 3 spaces × 2.5mm = 87.5mm > 60mm → needs 2 lines
        let boxes: Vec<WordBox> = (0..4).map(|_| WordBox { width: 20.0 }).collect();
        let breaks = opt.optimize(&boxes);
        assert_eq!(
            breaks.len(),
            2,
            "should produce exactly two lines: {breaks:?}"
        );
        assert_eq!(*breaks.last().unwrap(), 3);
    }

    #[test]
    fn last_break_is_always_last_word() {
        let opt = KnuthPlassOptimizer::new(50.0, 2.5);
        let boxes: Vec<WordBox> = (0..10).map(|_| WordBox { width: 15.0 }).collect();
        let breaks = opt.optimize(&boxes);
        assert_eq!(*breaks.last().unwrap(), 9);
    }
}
