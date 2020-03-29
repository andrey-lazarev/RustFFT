use num_complex::Complex;
use num_traits::Zero;

use std::sync::Arc;

use rand::{StdRng, SeedableRng};
use rand::distributions::{Normal, Distribution};

use algorithm::{DFT, butterflies};
use Fft;


/// The seed for the random number generator used to generate
/// random signals. It's defined here so that we have deterministic
/// tests
const RNG_SEED: [u8; 32] = [1, 9, 1, 0, 1, 1, 4, 3, 1, 4, 9, 8,
    4, 1, 4, 8, 2, 8, 1, 2, 2, 2, 6, 1, 2, 3, 4, 5, 6, 7, 8, 9];

pub fn random_signal(length: usize) -> Vec<Complex<f32>> {
    let mut sig = Vec::with_capacity(length);
    let normal_dist = Normal::new(0.0, 10.0);
    let mut rng: StdRng = SeedableRng::from_seed(RNG_SEED);
    for _ in 0..length {
        sig.push(Complex{re: (normal_dist.sample(&mut rng) as f32),
                         im: (normal_dist.sample(&mut rng) as f32)});
    }
    return sig;
}

pub fn compare_vectors(vec1: &[Complex<f32>], vec2: &[Complex<f32>]) -> bool {
    assert_eq!(vec1.len(), vec2.len());
    let mut sse = 0f32;
    for (&a, &b) in vec1.iter().zip(vec2.iter()) {
        sse = sse + (a - b).norm();
    }
    return (sse / vec1.len() as f32) < 0.1f32;
}

pub fn check_fft_algorithm(fft: &Fft<f32>, len: usize, inverse: bool) {
    assert_eq!(fft.len(), len, "Algorithm reported incorrect size");
    assert_eq!(fft.is_inverse(), inverse, "Algorithm reported incorrect inverse value");

    let n = 3;

    //test the forward direction
    let dft = DFT::new(len, inverse);

    // set up buffers
    let reference_input = random_signal(len * n);
    let mut expected_input = reference_input.clone();
    let mut expected_output = vec![Zero::zero(); len * n];
    dft.process_multi(&mut expected_input, &mut expected_output, &mut []);

    // test process()
    {
        let mut input = reference_input.clone();
        let mut output = expected_output.clone();

        for (input_chunk, output_chunk) in input.chunks_mut(len).zip(output.chunks_mut(len)) {
            fft.process(input_chunk, output_chunk);
        }
        assert!(compare_vectors(&expected_output, &output), "process() failed, length = {}, inverse = {}", len, inverse);
    }
    
    // test process_with_scratch()
    {
        let mut input = reference_input.clone();
        let mut scratch = vec![Zero::zero(); fft.get_out_of_place_scratch_len()];
        let mut output = expected_output.clone();

        for (input_chunk, output_chunk) in input.chunks_mut(len).zip(output.chunks_mut(len)) {
            fft.process_with_scratch(input_chunk, output_chunk, &mut scratch);
        }
        assert!(compare_vectors(&expected_output, &output), "process_with_scratch() failed, length = {}, inverse = {}", len, inverse);

        // make sure this algorithm works correctly with dirty scratch
        if scratch.len() > 0 {
            for item in scratch.iter_mut() {
                *item = Complex::new(100.0,100.0);
            }
            input.copy_from_slice(&reference_input);
            for (input_chunk, output_chunk) in input.chunks_mut(len).zip(output.chunks_mut(len)) {
                fft.process_with_scratch(input_chunk, output_chunk, &mut scratch);
            }

            assert!(compare_vectors(&expected_output, &output), "process_with_scratch() failed the 'dirty scratch' test, length = {}, inverse = {}", len, inverse);
        }
    }

    // test process_multi()
    {
        let mut input = reference_input.clone();
        let mut scratch = vec![Zero::zero(); fft.get_out_of_place_scratch_len()];
        let mut output = expected_output.clone();

        fft.process_multi(&mut input, &mut output, &mut scratch);
        assert!(compare_vectors(&expected_output, &output), "process_multi() failed, length = {}, inverse = {}", len, inverse);

        // make sure this algorithm works correctly with dirty scratch
        if scratch.len() > 0 {
            for item in scratch.iter_mut() {
                *item = Complex::new(100.0,100.0);
            }
            input.copy_from_slice(&reference_input);
            fft.process_multi(&mut input, &mut output, &mut scratch);

            assert!(compare_vectors(&expected_output, &output), "process_multi() failed the 'dirty scratch' test, length = {}, inverse = {}", len, inverse);
        }
    }

    // test process_inplace()
    {
        let mut buffer = reference_input.clone();

        for chunk in buffer.chunks_mut(len) {
            fft.process_inplace(chunk);
        }
        assert!(compare_vectors(&expected_output, &buffer), "process_inplace() failed, length = {}, inverse = {}", len, inverse);
    }
    
    // test process_inplace_with_scratch()
    {
        let mut buffer = reference_input.clone();
        let mut scratch = vec![Zero::zero(); fft.get_inplace_scratch_len()];

        for chunk in buffer.chunks_mut(len) {
            fft.process_inplace_with_scratch(chunk, &mut scratch);
        }
        assert!(compare_vectors(&expected_output, &buffer), "process_inplace_with_scratch() failed, length = {}, inverse = {}", len, inverse);

        // make sure this algorithm works correctly with dirty scratch
        if scratch.len() > 0 {
            for item in scratch.iter_mut() {
                *item = Complex::new(100.0,100.0);
            }
            buffer.copy_from_slice(&reference_input);
            for chunk in buffer.chunks_mut(len) {
                fft.process_inplace_with_scratch(chunk, &mut scratch);
            }
            assert!(compare_vectors(&expected_output, &buffer), "process_inplace_with_scratch() failed the 'dirty scratch' test, length = {}, inverse = {}", len, inverse);
        }
    }

    // test process_inplace_multi()
    {
        let mut buffer = reference_input.clone();
        let mut scratch = vec![Zero::zero(); fft.get_inplace_scratch_len()];

        fft.process_inplace_multi(&mut buffer, &mut scratch);
        assert!(compare_vectors(&expected_output, &buffer), "process_inplace_multi() failed, length = {}, inverse = {}", len, inverse);

        // make sure this algorithm works correctly with dirty scratch
        if scratch.len() > 0 {
            for item in scratch.iter_mut() {
                *item = Complex::new(100.0,100.0);
            }
            buffer.copy_from_slice(&reference_input);
            fft.process_inplace_multi(&mut buffer, &mut scratch);

            assert!(compare_vectors(&expected_output, &buffer), "process_inplace_multi() failed the 'dirty scratch' test, length = {}, inverse = {}", len, inverse);
        }
    }
}

pub fn make_butterfly(len: usize, inverse: bool) -> Arc<butterflies::FFTButterfly<f32>> {
    match len {
        2 => Arc::new(butterflies::Butterfly2::new(inverse)),
        3 => Arc::new(butterflies::Butterfly3::new(inverse)),
        4 => Arc::new(butterflies::Butterfly4::new(inverse)),
        5 => Arc::new(butterflies::Butterfly5::new(inverse)),
        6 => Arc::new(butterflies::Butterfly6::new(inverse)),
        7 => Arc::new(butterflies::Butterfly7::new(inverse)),
        8 => Arc::new(butterflies::Butterfly8::new(inverse)),
        16 => Arc::new(butterflies::Butterfly16::new(inverse)),
        _ => panic!("Invalid butterfly size: {}", len),
    }
}
