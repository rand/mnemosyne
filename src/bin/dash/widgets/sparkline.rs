//! Sparkline widget for inline time-series visualization

use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Unicode block characters for sparklines (8 levels)
const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Compact sparkline visualization for time-series data
pub struct Sparkline<'a> {
    data: &'a [f32],
    style: Style,
    width: usize,
}

impl<'a> Sparkline<'a> {
    /// Create new sparkline with data
    pub fn new(data: &'a [f32]) -> Self {
        Self {
            data,
            style: Style::default(),
            width: 7, // Default compact width
        }
    }

    /// Set custom style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set custom width (number of characters)
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Render sparkline as a single line of text
    pub fn render(&self) -> Line<'a> {
        if self.data.is_empty() {
            // Empty data - show placeholder
            return Line::from(Span::styled("─".repeat(self.width), self.style));
        }

        if self.data.len() == 1 {
            // Single point - show single block at middle height
            let block = BLOCKS[4]; // Middle height
            return Line::from(Span::styled(
                format!("{}{}", block, "─".repeat(self.width.saturating_sub(1))),
                self.style,
            ));
        }

        // Find min/max for scaling, filtering out NaN and infinity
        let finite_values: Vec<f32> = self
            .data
            .iter()
            .copied()
            .filter(|x| x.is_finite())
            .collect();

        // If all values are non-finite, show placeholder
        if finite_values.is_empty() {
            return Line::from(Span::styled("─".repeat(self.width), self.style));
        }

        let min = finite_values
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        let max = finite_values
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(1.0);

        let range = max - min;

        // If all values are the same, show flat line at middle height
        if range < f32::EPSILON {
            let block = BLOCKS[4];
            return Line::from(Span::styled(block.to_string().repeat(self.width), self.style));
        }

        // Sample or interpolate data to fit width
        let display_data = self.sample_data();

        // Scale each value to block character index (0-8)
        let chars: String = display_data
            .iter()
            .map(|&value| {
                // Handle non-finite values gracefully
                if !value.is_finite() {
                    return ' '; // Use space for invalid values
                }
                let normalized = (value - min) / range;
                let index = (normalized * 8.0).round() as usize;
                BLOCKS[index.min(8)]
            })
            .collect();

        Line::from(Span::styled(chars, self.style))
    }

    /// Sample data points to fit target width
    fn sample_data(&self) -> Vec<f32> {
        if self.data.len() <= self.width {
            // Use all points, pad if needed
            let mut result = self.data.to_vec();
            while result.len() < self.width {
                result.insert(0, *result.first().unwrap_or(&0.0));
            }
            result
        } else {
            // Downsample by taking evenly-spaced points
            let step = self.data.len() as f32 / self.width as f32;
            (0..self.width)
                .map(|i| {
                    let index = (i as f32 * step) as usize;
                    self.data[index.min(self.data.len() - 1)]
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_data() {
        let sparkline = Sparkline::new(&[]);
        let line = sparkline.render();
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "───────"); // Default width is 7
    }

    #[test]
    fn test_single_point() {
        let sparkline = Sparkline::new(&[5.0]);
        let line = sparkline.render();
        assert!(line.spans[0].content.starts_with('▄'));
    }

    #[test]
    fn test_flat_line() {
        let sparkline = Sparkline::new(&[3.0, 3.0, 3.0, 3.0]);
        let line = sparkline.render();
        // All same value should show middle height blocks
        assert!(line.spans[0].content.chars().all(|c| c == '▄'));
    }

    #[test]
    fn test_increasing_trend() {
        let sparkline = Sparkline::new(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let line = sparkline.render();
        let chars: Vec<char> = line.spans[0].content.chars().collect();
        // Should show increasing pattern
        for i in 0..chars.len() - 1 {
            let curr_idx = BLOCKS.iter().position(|&c| c == chars[i]).unwrap();
            let next_idx = BLOCKS.iter().position(|&c| c == chars[i + 1]).unwrap();
            assert!(next_idx >= curr_idx);
        }
    }

    #[test]
    fn test_decreasing_trend() {
        let sparkline = Sparkline::new(&[5.0, 4.0, 3.0, 2.0, 1.0]);
        let line = sparkline.render();
        let chars: Vec<char> = line.spans[0].content.chars().collect();
        // Should show decreasing pattern
        for i in 0..chars.len() - 1 {
            let curr_idx = BLOCKS.iter().position(|&c| c == chars[i]).unwrap();
            let next_idx = BLOCKS.iter().position(|&c| c == chars[i + 1]).unwrap();
            assert!(next_idx <= curr_idx);
        }
    }

    #[test]
    fn test_custom_width() {
        let sparkline = Sparkline::new(&[1.0, 2.0, 3.0]).width(10);
        let line = sparkline.render();
        assert_eq!(line.spans[0].content.chars().count(), 10);
    }

    #[test]
    fn test_downsampling() {
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let sparkline = Sparkline::new(&data).width(7);
        let line = sparkline.render();
        // Should downsample to 7 characters
        assert_eq!(line.spans[0].content.chars().count(), 7);
    }

    #[test]
    fn test_nan_values() {
        // Should not panic with NaN values
        let sparkline = Sparkline::new(&[1.0, f32::NAN, 3.0, 4.0, 5.0]);
        let line = sparkline.render();
        // Should render successfully (NaN replaced with space)
        assert_eq!(line.spans[0].content.chars().count(), 7); // Default width
    }

    #[test]
    fn test_all_nan_values() {
        // Should not panic when all values are NaN
        let sparkline = Sparkline::new(&[f32::NAN, f32::NAN, f32::NAN]);
        let line = sparkline.render();
        // Should show placeholder
        assert_eq!(line.spans[0].content, "───────");
    }

    #[test]
    fn test_infinity_values() {
        // Should not panic with infinity values
        let sparkline = Sparkline::new(&[1.0, f32::INFINITY, 3.0, f32::NEG_INFINITY, 5.0]);
        let line = sparkline.render();
        // Should render successfully (infinities replaced with space)
        assert_eq!(line.spans[0].content.chars().count(), 7);
    }

    #[test]
    fn test_mixed_finite_and_nan() {
        // Should render finite values correctly, ignoring NaN
        let sparkline = Sparkline::new(&[1.0, 2.0, f32::NAN, 4.0, 5.0]);
        let line = sparkline.render();
        // Should show increasing pattern for finite values
        assert_eq!(line.spans[0].content.chars().count(), 7);
        // Should not contain error indicators in finite value positions
        let chars: Vec<char> = line.spans[0].content.chars().collect();
        // The NaN should be rendered as space in its position
        assert!(chars.contains(&' '));
    }
}
