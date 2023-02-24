// I'm not quite sure I understand the task at hand...

// Filled Julia Set - Numberphile2
// https://www.youtube.com/watch?v=oCkQ7WK7vuY

// Mandelbrot Set Explained Series - The Mathemagicians' Guild
// https://www.youtube.com/playlist?list=PL9tHLTl03LqG4ajDvqyfCDMKSxmR_plJ3
// (Though most of this went way over my head)

use rand::prelude::Distribution;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WithProofOfWork<T> {
    pub candidate: num::Complex<f64>,
    pub inner: T,
}

pub fn do_work(
    c: num::Complex<f64>,
    re_min: f64,
    re_max: f64,
    target_iterations: u16,
) -> num::Complex<f64> {
    let mut rng0 = rand::thread_rng();
    let mut rng1 = rand::thread_rng();
    let re_distribution = rand::distributions::Uniform::new(re_min, re_max);
    let im_distribution = rand::distributions::Uniform::new(-1.0, 1.0);
    for re in re_distribution.sample_iter(&mut rng0) {
        for im in im_distribution.sample_iter(&mut rng1) {
            if let Ok(found) = check_work(
                c,
                re_min,
                re_max,
                num::Complex { re, im },
                target_iterations,
            ) {
                return found;
            }
        }
    }
    unreachable!()
}

fn iterate_julia(c: num::Complex<f64>, z: num::Complex<f64>) -> num::Complex<f64> {
    z.powu(2) + c
}

// is julia set continuous? Can we do gradient traversal?
// how can we deal with floating point errors?
pub fn check_work(
    c: num::Complex<f64>,
    re_min: f64,
    re_max: f64,
    candidate: num::Complex<f64>,
    target_iterations: u16,
) -> Result<num::Complex<f64>, DoWorkError> {
    let mut current = candidate;
    for iteration in 0..=target_iterations {
        current = iterate_julia(c, current);
        if current.re < re_min || current.re > re_max {
            match iteration == target_iterations {
                true => return Ok(candidate),
                false => return Err(DoWorkError::LeftSetTooEarly),
            }
        }
    }
    Err(DoWorkError::LeftSetTooLateOrNotAtAll)
}

pub enum DoWorkError {
    LeftSetTooEarly,
    LeftSetTooLateOrNotAtAll,
}
