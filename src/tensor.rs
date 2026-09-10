// TODO
//
// Some of the below can be replaced with operator impls if i can ever be bothered;
// see [CAB401 raytracer implementation]!!
//
// [CAB401 raytracer implementation]: https://github.com/plsuwu/CAB401/tree/master/a2

pub struct Mat {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f32>,
}

impl Mat {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    pub fn from_vec(rows: usize, cols: usize, data: Vec<f32>) -> Self {
        assert_eq!(data.len(), rows * cols, "shape mismatch");
        Self { rows, cols, data }
    }

    #[inline]
    pub fn row(&self, i: usize) -> &[f32] {
        &self.data[i * self.cols..(i + 1) * self.cols]
    }

    #[inline]
    pub fn row_mut(&mut self, i: usize) -> &mut [f32] {
        &mut self.data[i * self.cols..(i + 1) * self.cols]
    }

    /// `self[n,k] * w[m,k]^T -> [n,m]`
    pub fn matmul_t(&self, w: &Mat) -> Mat {
        assert_eq!(self.cols, w.cols, "matmul_t inner dim mismatch");
        let (n, m, k) = (self.rows, w.rows, self.cols);
        let mut out = vec![0.0f32; n * m];

        let threads = std::thread::available_parallelism()
            .map(|t| t.get())
            .unwrap_or(1);
        let block = (n + threads - 1) / threads.max(1);
        if block == 0 {
            return Mat::from_vec(n, m, out);
        }

        std::thread::scope(|s| {
            for (bi, out_chunk) in out.chunks_mut(block * m).enumerate() {
                let x = &self.data;
                let wd = &w.data;
                s.spawn(move || {
                    let r0 = bi * block;
                    for (ri, orow) in out_chunk.chunks_mut(m).enumerate() {
                        let xrow = &x[(r0 + ri) * k..(r0 + ri + 1) * k];
                        for (j, o) in orow.iter_mut().enumerate() {
                            *o = dot(xrow, &wd[j * k..(j + 1) * k]);
                        }
                    }
                });
            }
        });
        Mat::from_vec(n, m, out)
    }

    pub fn add_row_bias(&mut self, b: &[f32]) {
        assert_eq!(b.len(), self.cols);
        for r in 0..self.rows {
            for (x, y) in self.row_mut(r).iter_mut().zip(b) {
                *x += y;
            }
        }
    }

    pub fn add(&mut self, other: &Mat) {
        assert_eq!(self.data.len(), other.data.len());
        for (x, y) in self.data.iter_mut().zip(&other.data) {
            *x += y;
        }
    }

    pub fn map_inplace(&mut self, f: impl Fn(f32) -> f32) {
        for x in &mut self.data {
            *x = f(*x);
        }
    }
}

#[inline]
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    // 8 accumulators so the compiler can auto-vectorize
    let mut acc = [0.0f32; 8];
    let chunks = a.len() / 8;
    for i in 0..chunks {
        for l in 0..8 {
            acc[l] += a[i * 8 + l] * b[i * 8 + l];
        }
    }
    let mut s: f32 = acc.iter().sum();
    for i in chunks * 8..a.len() {
        s += a[i] * b[i];
    }
    s
}

pub fn layer_norm(x: &mut [f32], gamma: &[f32], beta: &[f32], eps: f32) {
    let n = x.len() as f32;
    let mean = x.iter().sum::<f32>() / n;
    let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n;
    let inv = 1.0 / (var + eps).sqrt();
    for i in 0..x.len() {
        x[i] = (x[i] - mean) * inv * gamma[i] + beta[i];
    }
}

pub fn softmax(x: &mut [f32]) {
    let max = x.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0;
    for v in x.iter_mut() {
        *v = (*v - max).exp();
        sum += *v;
    }
    for v in x.iter_mut() {
        *v /= sum;
    }
}

pub fn gelu(x: f32) -> f32 {
    0.5 * x * (1.0 + erf(x / std::f32::consts::SQRT_2))
}

fn erf(x: f32) -> f32 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0
        - (((((1.061_405_4 * t - 1.453_152_1) * t) + 1.421_413_8) * t - 0.284_496_72) * t
            + 0.254_829_6)
            * t
            * (-x * x).exp();
    sign * y
}

pub fn l2_normalize(v: &mut [f32]) {
    let n = dot(v, v).sqrt().max(1e-12);
    for x in v {
        *x /= n;
    }
}
