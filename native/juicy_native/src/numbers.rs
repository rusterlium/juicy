use ::rustler::{
    NifEnv,
    NifTerm,
    NifEncoder,
};

use ::num_traits::Num;
use ::num_bigint::BigUint;

use ::std::str::FromStr;

use ::iterative_json_parser::{NumberData, Range};

fn integer_to_bigint_term<'a>(env: NifEnv<'a>, sign: bool, number: &str) -> NifTerm<'a> {
    // http://erlang.org/doc/apps/erts/erl_ext_dist.html#id101259

    let num = BigUint::from_str_radix(number, 10).unwrap();
    let bytes_le = num.to_bytes_le();
    let num_len = bytes_le.len();

    let mut buf = Vec::<u8>::with_capacity(num_len + 7);
    buf.push(131); // magic
    buf.push(111); // large bignum tag

    let number_len_bytes: [u8; 4] = [
        (num_len >> 24) as u8,
        (num_len >> 16) as u8,
        (num_len >> 8) as u8,
        (num_len >> 0) as u8,
    ];
    buf.extend_from_slice(&number_len_bytes);

    buf.push(if sign { 0 } else { 1 });

    buf.extend_from_slice(&bytes_le);

    // This is safe because we manually constructed the data, and we
    // are completely sure that it is valid.
    let (term, _) = unsafe { env.binary_to_term_trusted(&buf) }.unwrap();
    term
}

fn integer_to_term<'a>(env: NifEnv<'a>, sign: bool, num_str: &str) -> NifTerm<'a> {
    if sign {
        match u64::from_str(num_str) {
            Ok(number) => number.encode(env),
            Err(_) => integer_to_bigint_term(env, sign, num_str),
        }
    } else {
        match i64::from_str(num_str) {
            Ok(number) => (-number).encode(env),
            Err(_) => integer_to_bigint_term(env, sign, num_str),
        }
    }
}

fn float_to_term<'a>(env: NifEnv<'a>, num_str: &str) -> NifTerm<'a> {
    let number = f64::from_str(num_str).ok().unwrap();
    number.encode(env)
}

pub fn number_data_to_term<'a, F>(env: NifEnv<'a>, data: NumberData, range_provider: F) -> NifTerm<'a>
    where F: Fn(Range, &mut Vec<u8>) {

    // TODO: Do not allocate
    let mut buf = Vec::<u8>::new();

    match data {
        NumberData { decimal: None, exponent: None, .. } => {
            // This is safe because the tokenizer only accepts digits when reading numbers.
            // This byte range will thus never contain anything other than characters 0...9.
            range_provider(data.integer, &mut buf);
            let num_str = unsafe { ::std::str::from_utf8_unchecked(&buf) };

            integer_to_term(env, data.sign, num_str)
        }
        _ => {
            if !data.sign {
                buf.push('-' as u8);
            }
            range_provider(data.integer, &mut buf);
            if let Some(decimal) = data.decimal {
                buf.push('.' as u8);
                range_provider(decimal, &mut buf);
            }
            if let Some(exponent) = data.exponent {
                buf.push('e' as u8);
                if !data.exponent_sign {
                    buf.push('-' as u8);
                }
                range_provider(exponent, &mut buf);
            }

            // This is safe because the tokenizer only accepts digits when reading numbers.
            // This range will thus never contain anything other than 0..9 + the symbols
            // we added.
            let num_str = unsafe { ::std::str::from_utf8_unchecked(&buf) };

            float_to_term(env, num_str)
        }
    }
}
