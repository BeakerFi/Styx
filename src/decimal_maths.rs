/** This

**/

use std::fmt;
use std::ops::Neg;
use scrypto::dec;
use scrypto::prelude::{Decimal, I256, I512};
use num_bigint::{BigInt};

pub const EULER_CONST: Decimal = Decimal(I256([
    0x6A, 0x61, 0xB3, 0xC0, 0xEB, 0x46, 0xB9, 0x25, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00,
]));

pub const SQRT_MAX: Decimal = Decimal(I256([
    0x85, 0xED, 0xE9, 0x51, 0x72, 0x63, 0xA4, 0x40, 0xDE, 0x32, 0x8E, 0x73, 0x1C, 0x94, 0xC1,
    0x2F, 0xDD, 0x97, 0x25, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00,
]));

/// Returns the exponential of a [`Decimal`] using Taylor series
///
/// # Arguments
/// * `value` - The Decimal to compute the exponential for
///
/// # Examples
///
/// ```
/// use scrypto::prelude::Decimal;
/// use styx::decimal_maths::exp;
/// let res = exp(Decimal::one());
/// let true_res = Decimal::from(1.0_f64.exp().to_string());
/// let diff = res - true_res;
/// assert!(diff.abs() < Decimal::from("0.000000000000001"));
/// ```
pub fn exp<T: TryInto<Decimal>>(value: T) -> Decimal
    where <T as TryInto<Decimal>>::Error: fmt::Debug,
{
    let value = value.try_into().expect("Cannot convert to Decimal");
    if value.is_zero()
    {
        return Decimal::ONE;
    }
    else if value.is_negative()
    {
        return if value < dec!(-43)
        {
            // In this case, we return because exp(-43) < 10^-18 and exp(value) could overflow
            Decimal::zero()
        } else {
            Decimal::ONE / exp(value.neg())
        }
    }
    else
    {
        // outputted result
        let mut result = Decimal::one();
        // next term of the taylor expansion
        let mut added_term = value;
        // counter to remember the index of the next term to add
        let mut counter = Decimal::one();
        while added_term != Decimal::zero()
        {
            result = result + added_term;
            counter = counter + 1;
            let mut next_term = added_term /counter ;
            next_term = next_term * value;
            added_term = next_term;
        }

        result
    }

}

/// Returns the natural logarithm of a [`Decimal`] using Halley's method
///
/// # Arguments
/// * `value` - The Decimal to compute the logarithm for
///
/// # Examples
///
/// ```
/// use scrypto::prelude::Decimal;
/// use styx::decimal_maths::ln;
///
/// let res = ln(100);
/// let true_res = Decimal::from(100.0_f64.ln().to_string());
/// let diff = res - true_res;
/// assert!(diff.abs() < Decimal::from("0.000000000000001"));
/// ```
pub fn ln<T: TryInto<Decimal>>(value: T) -> Decimal
    where <T as TryInto<Decimal>>::Error: fmt::Debug,
{
    let mut value = value.try_into().expect("Cannot convert to Decimal");
    assert!(value.is_positive(), "Logarithm is only defined for positive numbers");

    // We rewrite value = x*e^n with x < e
    // Therefore, ln(value) = ln(x) + n

    let mut n = 0;
    while value > EULER_CONST
    {
        value = value / EULER_CONST;
        n += 1;
    }

    // Start with an arbitrary number as the first guess
    let mut result = value / Decimal::from(2u8);

    // Too small to represent, so we start with self
    // Future iterations could actually avoid using a decimal altogether and use a buffered
    // vector, only combining back into a decimal on return
    if result.is_zero() {
        result = value;
    }
    let mut last = result + 1;

    // Keep going while last and result are not equal
    let mut circuit_breaker = 0;
    while last != result {
        circuit_breaker += 1;
        assert!(circuit_breaker < 1000, "geo mean circuit breaker");

        last = result;
        let exp_last = exp(last);
        result = last + (value - exp_last)/(value + exp_last)*2;
    }

    result + Decimal::from(n)
}

/// Returns the 3rd root of a [`Decimal`] using Newton's method
///
/// # Arguments
///
/// * `value` - The Decimal to compute the 3rd root for
///
/// # Examples
///
/// ```
/// use scrypto::dec;
/// use scrypto::prelude::Decimal;
/// use styx::decimal_maths::sqrt_3;
///
/// let res = sqrt_3(27);
/// let true_res = dec!(3);
/// assert_eq!(true_res, res);
/// ```
pub fn sqrt_3<T: TryInto<Decimal>>(value:T) -> Decimal
    where <T as TryInto<Decimal>>::Error: fmt::Debug
{
    let value = value.try_into().expect("Cannot convert to Decimal");

    if value == Decimal::one() || value == Decimal::zero()
    {
        return value
    }

    // Because we will using squares, we need our initial guess to be small enough not to overflow.
    // Hence, if it is too big, we start by sqrt(Decimal::MAX)/2

    let sgn = if value.is_positive() { 1 } else { -1 };
    let mut result = if value.abs() >= SQRT_MAX { SQRT_MAX/2*sgn }
                             else { value/2 };

    // Too small to represent, so we start with self
    // Future iterations could actually avoid using a decimal altogether and use a buffered
    // vector, only combining back into a decimal on return
    if result.is_zero() {
        result = value;
    }

    let mut last = result - 1;
    // Keep going while last and result are not equal
    let mut circuit_breaker = 0;
    while last != result
    {
        circuit_breaker += 1;
        assert!(circuit_breaker < 1000, "geo mean circuit breaker");

        last = result;
        result = ( result*2 + value / (result*result) )/3;

    }

    result

}


pub fn cbrt<T: TryInto<Decimal>>(value:T) -> Decimal
    where <T as TryInto<Decimal>>::Error: fmt::Debug
{
    let value = value.try_into().expect("Cannot convert to Decimal");

    if value.abs() == Decimal::one() || value == Decimal::zero()
    {
        return value
    }

    // In BigInt the number is represented by x*10^18. Hence, taking the cubic root yields
    // cbrt(x) * 10^6 and we get only 6 decimals. We multiply by 10^36 to get the right precision.
    // In fact cbrt[(x*10^18*10^36)] = cbrt(x) * 10^(6 + 12) = cbrt(x) * 10^18
    let pow_10_36 : I512 = I512::from(dec!("1000000000000000000000000000000000000").0 / Decimal::one().0) ;
    let tmp_1 = BigInt::from(I512::from(value.0)) * BigInt::from(pow_10_36);


    let result: I512 = tmp_1.cbrt().try_into().unwrap();

    Decimal(I256::try_from(result).unwrap())
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use scrypto::dec;
    use scrypto::math::Decimal;
    use crate::decimal_maths::{exp, ln, cbrt, SQRT_MAX, sqrt_3};

    #[test]
    fn test_exp_zero() {
        let res = exp(0);
        let true_res = Decimal::one();
        assert_eq!(res,true_res);
    }

    #[test]
    fn test_exp_random_pos() {
        let num: f64 = rand::thread_rng().gen_range(0.0..2.0);
        let dec_num = Decimal::from(num.to_string());
        let res = exp(dec_num);
        let true_res = Decimal::from(num.exp().to_string());
        let diff = res - true_res;
        assert!(diff.abs() < Decimal::from("0.000000000000001"));
    }

    #[test]
    fn test_exp_random_neg() {
        let num: f64 = rand::thread_rng().gen_range(-2.0..0.0);
        let dec_num = Decimal::from(num.to_string());
        let res = exp(dec_num);
        let true_res = Decimal::from(num.exp().to_string());
        let diff = res - true_res;
        assert!(diff.abs() < Decimal::from("0.000000000000001"));
    }

    #[test]
    #[should_panic]
    fn test_ln_neg()
    {
        let _m = ln(-5);
    }

    #[test]
    #[should_panic]
    fn test_ln_zero()
    {
        let _m = ln(0);
    }

    #[test]
    fn test_ln_int()
    {
        let res = ln(exp(12));
        let true_res = dec!(12);
        let diff = res - true_res;
        assert!(diff.abs() < Decimal::from("0.000000000000001"));
    }

    #[test]
    fn test_ln_random()
    {
        let num: f64 = rand::thread_rng().gen_range(0.0..10000.0);
        let dec_num = Decimal::from(num.to_string());
        let res = ln(dec_num);
        let true_res = Decimal::from(num.ln().to_string());
        let diff = res - true_res;
        assert!(diff.abs() < Decimal::from("0.000000000000001"), "num: {}, res: {}, true_res {}", num, res, true_res);
    }

    #[test]
    fn test_sqrt_3_int()
    {
        let res = cbrt(729);
        let true_res = dec!(9);
        assert_eq!(true_res, res);
    }

    #[test]
    fn test_sqrt_3_neg_int()
    {
        let res = cbrt(-729);
        let true_res = dec!(-9);
        assert_eq!(true_res, res);
    }

    #[test]
    fn test_sqrt_3_random()
    {
        let range: f64 = 10e10;
        let num : f64 =  rand::thread_rng().gen_range(-range..range);
        let dec_num = Decimal::from(num.to_string());
        let res = cbrt(dec_num);
        let true_res_f = num.powf(1.0 / 3.0 );
        let true_res = Decimal::from(true_res_f.to_string());

        let other_res = sqrt_3(dec_num);
        let pow3 = res.powi(3);
        let diff = res - true_res;
        assert!(diff.abs() < Decimal::from("0.000000000000001"), "num: {}, res: {}, true_res {}, sqrt_3: {}, pow: {}", num, res, true_res, other_res, pow3);
    }



}