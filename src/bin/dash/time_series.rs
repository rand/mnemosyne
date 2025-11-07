//! Time-series data collection and storage

use std::collections::VecDeque;

/// Ring buffer for fixed-size time-series data storage
#[derive(Debug, Clone)]
pub struct TimeSeriesBuffer<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T> TimeSeriesBuffer<T> {
    /// Create new buffer with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push new value, removing oldest if at capacity
    pub fn push(&mut self, value: T) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    /// Get current number of stored values
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get all values as slice
    pub fn as_slice(&self) -> Vec<&T> {
        self.data.iter().collect()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if buffer is at capacity
    pub fn is_full(&self) -> bool {
        self.data.len() >= self.capacity
    }
}

impl<T: Copy> TimeSeriesBuffer<T> {
    /// Get values as owned Vec (for types that implement Copy)
    pub fn to_vec(&self) -> Vec<T> {
        self.data.iter().copied().collect()
    }
}

impl<T> Default for TimeSeriesBuffer<T> {
    fn default() -> Self {
        Self::new(50) // Default capacity of 50 points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer: TimeSeriesBuffer<i32> = TimeSeriesBuffer::new(10);
        assert_eq!(buffer.capacity(), 10);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_push() {
        let mut buffer = TimeSeriesBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        assert_eq!(buffer.len(), 3);
        assert!(buffer.is_full());
        assert_eq!(buffer.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut buffer = TimeSeriesBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4); // Should evict 1

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.to_vec(), vec![2, 3, 4]);
    }

    #[test]
    fn test_continuous_overflow() {
        let mut buffer = TimeSeriesBuffer::new(3);
        for i in 1..=10 {
            buffer.push(i);
        }

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.to_vec(), vec![8, 9, 10]);
    }

    #[test]
    fn test_clear() {
        let mut buffer = TimeSeriesBuffer::new(5);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        buffer.clear();

        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_default() {
        let buffer: TimeSeriesBuffer<f32> = TimeSeriesBuffer::default();
        assert_eq!(buffer.capacity(), 50);
    }

    #[test]
    fn test_float_values() {
        let mut buffer = TimeSeriesBuffer::new(5);
        buffer.push(1.5_f32);
        buffer.push(2.7_f32);
        buffer.push(3.2_f32);

        let values = buffer.to_vec();
        assert_eq!(values.len(), 3);
        assert!((values[0] - 1.5_f32).abs() < f32::EPSILON);
        assert!((values[1] - 2.7_f32).abs() < f32::EPSILON);
        assert!((values[2] - 3.2_f32).abs() < f32::EPSILON);
    }
}
