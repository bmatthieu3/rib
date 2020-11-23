const fn num_bits<T>() -> usize { std::mem::size_of::<T>() * 8 }

pub fn log_2(x: i32) -> u32 {
    assert!(x > 0);
    num_bits::<i32>() as u32 - x.leading_zeros() - 1
}