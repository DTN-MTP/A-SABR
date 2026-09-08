use crate::types::{Date, Duration, Volume};
use core::str::FromStr;

#[derive(Debug, Clone)]
pub struct Polynome<const N: usize> {
    pub coefficients: [i64; N],
    pub offset: Date
}

impl<const N: usize> Polynome<N>{
    pub fn new(coefficients: [i64;N], offset: Date) -> Self{
        Self {coefficients, offset}
    }

    pub fn evaluate_exact_integral(&self, x:Date) -> Volume{
        let x_offset: Duration = x - self.offset;
        self.coefficients
            .iter()
            .enumerate()
            .fold(0, |acc, (i, &c)| {
                // If the coefficient is 0, the power is not calculated to avoid an unnecessary overflow
                if c == 0 {
                    acc
                } else {
                    acc + (c * self.pow_simple(x_offset, i + 1)) / (i as i64 + 1)
                }
            })
    }

    fn pow_simple(&self, base:Duration, exp:usize) -> i64{
        let mut res = 1;
        for _ in 0..exp {
            res *= base;
        }
        res
    }

    pub fn find_end_bundle(&self, tx:Date, rx:Date, x:Date, i_target:Volume) -> Option<Date> {
        if x < tx || x >= rx {
            return None;
        }
        let f_x = self.evaluate_exact_integral(x);
        let f_rx = self.evaluate_exact_integral(rx);
        let available_area: Volume = f_rx - f_x;

        if i_target >= 0 && available_area < i_target{
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

impl<const N: usize> FromStr for Polynome<N> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Search for brackets containing the coefficients.
        let start = s.find('[').ok_or(())?;
        let end = s.find(']').ok_or(())?;
        
        let coeffs_str = &s[start + 1..end];
        let mut coefficients = [0; N];
        
        // Extraction and conversion of coefficients
        for (i, coeff) in coeffs_str.split_whitespace().enumerate() {
            if i < N {
                coefficients[i] = coeff.parse().map_err(|_| ())?;
            }
        }
        
        // The offset is located after the closing bracket.
        let offset_str = s[end + 1..].trim();
        let offset = offset_str.parse().map_err(|_| ())?;
        
        Ok(Polynome::new(coefficients, offset))
    }
}

impl<const N: usize> TryFrom<&str> for Polynome<N> {
    type Error = ();
    
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}