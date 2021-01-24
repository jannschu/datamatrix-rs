use crate::errorcode::GF;

fn find_error_locations(syndomes: &[GF]) -> Result<Vec<u8>, DecodingError> {
    // step 1: determine the coefficients of the error locator polynomial,
    // exploit that the syndrome matrix is a Hankel matrix, 
    // cf. https://arxiv.org/pdf/1310.2473.pdf
    let t = syndomes.len() / 2;

    let mut matrix = vec![GF(0); v * v]; // row major order
    let mut rhs = vec![GF(0); v];
    for i in 0..v {
        for j in 0..v {
            matrix[i * v + j] = syndomes[i + j];
        }
        rhs[i] = -syndomes[v + i];
    }

    let i0 = syndomes.iter().take_while(|s| **s == GF(0)).count();
    if i0 == syndomes.len() {
        return Ok(vec![]);
    }
    if i0 >= t {
        return Err(DecodingError::TooManyErrors);
    }

    // initialize y
    let mut y = Vec::with_capacity(t + 1);
    y.resize(i0 + 1, GF(0));
    y[0] = GF(1) / syndomes[i0];
    
    // initialize w, solve triangular system
    let mut w = Vec::with_capacity(t + 1);
    w.extend_from_slice(&syndomes[i0 + 1..2 * i0 + 1]);
    for i in 0..i0 + 1 {
        for j in i0 - i..i0 + 1 {
            let w_j = w[j];
            w[i] -= syndomes[j + i] * w_j;
        }
        w[i] /= syndomes[i0 + i];
    }

    let mut i = i0;
    while i + 1 < t {
        let eps_i = syndomes[i+1..2 * i + 1].iter().zip(w.iter()).map(|(s, w)| *s * *w).sum::<GF>() + syndomes[2*i + 1];
        if eps_i != GF(0) {
            let eps_i_inv = GF(1) / eps_i;
            y.push()
        }
    }

    // step 2: find the zeros of the error locator polynomial

    todo!()
}