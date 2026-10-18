use crate::types::{Date, Duration, Volume};

#[derive(Debug, Clone)]
pub struct Polynome<const N: usize> {
    pub coefficients: [i64; N],
    pub offset: Date,
}

impl<const N: usize> Polynome<N> {
    pub fn new(coefficients: [i64; N], offset: Date) -> Self {
        Self {
            coefficients,
            offset,
        }
    }

    pub fn evaluate_exact_integral(&self, x: Date) -> Volume {
        let x_offset: Duration = x - self.offset;
        if x_offset == 0 {
            return 0;
        }

        // Horner (from right to left)
        let inner: i64 = self.coefficients
            .iter()
            .enumerate()
            .rfold(0, |acc, (i, &c)| {
                // acc * x + (c / (i + 1))
                (acc * x_offset) + (c / (i as i64 + 1))
            });

        inner * x_offset
    }

    fn pow_simple(&self, base: Duration, exp: usize) -> i64 {
        let mut res = 1;
        for _ in 0..exp {
            res *= base;
        }
        res
    }

    pub fn find_end_bundle(&self, tx: Date, rx: Date, x: Date, i_target: Volume) -> Option<Date> {
        if x < tx || x >= rx {
            return None;
        }
        let f_x = self.evaluate_exact_integral(x);
        let f_rx = self.evaluate_exact_integral(rx);
        let available_area: Volume = f_rx - f_x;

        if i_target >= 0 && available_area < i_target {
            return None;
        }
        if i_target < 0 && available_area > i_target {
            return None;
        }

        let mut lower_bound: Date = x;
        let mut upper_bound: Date = rx;

        while lower_bound <= upper_bound {
            let y = lower_bound + (upper_bound - lower_bound) / 2;
            let current_area: Volume = self.evaluate_exact_integral(y) - f_x;

            if current_area == i_target {
                return Some(y);
            }

            if current_area < i_target {
                lower_bound = y + 1;
            } else {
                upper_bound = y - 1;
            }
        }

        Some(lower_bound)
    }
}
