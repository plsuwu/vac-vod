pub mod batched;
pub mod gpu;

use crate::safetensors::SafeTensors;
use crate::tensor::{Mat, dot, gelu, l2_normalize, layer_norm, softmax};

#[derive(Clone, Copy)]
pub enum Pooling {
    /// all-MiniLM-, all-mpnet-
    Mean,
    /// bge-, e5-
    Cls,
}

#[derive(Clone, Copy)]
pub struct Config {
    pub hidden: usize,
    pub heads: usize,
    pub layers: usize,
    pub intermediate: usize,
    pub eps: f32,
    pub pooling: Pooling,
}

impl Config {
    pub fn minilm_l6() -> Self {
        Self {
            hidden: 384,
            heads: 12,
            layers: 6,
            intermediate: 1536,
            eps: 1e-12,
            pooling: Pooling::Mean,
        }
    }

    pub fn e5_large() -> Self {
        Self {
            hidden: 1024,
            heads: 16,
            layers: 24,
            intermediate: 4096,
            eps: 1e-12,
            pooling: Pooling::Mean,
        }
    }

    pub fn gte_base() -> Self {
        Self {
            hidden: 768,
            heads: 12,
            layers: 12,
            intermediate: 3072,
            eps: 1e-12,
            pooling: Pooling::Mean,
        }
    }

    pub fn gte_small() -> Self {
        Self {
            hidden: 384,
            heads: 12,
            layers: 12,
            intermediate: 1536,
            eps: 1e-12,
            pooling: Pooling::Mean,
        }
    }
}

pub struct Linear {
    w: Mat, // [out, in]
    b: Vec<f32>,
}

impl Linear {
    fn load(st: &SafeTensors, prefix: &str) -> Self {
        let (shape, w) = get(st, &format!("{prefix}.weight"));
        let (_, b) = get(st, &format!("{prefix}.bias"));

        Self {
            w: Mat::from_vec(shape[0], shape[1], w),
            b,
        }
    }

    fn forward(&self, x: &Mat) -> Mat {
        let mut y = x.matmul_t(&self.w);
        y.add_row_bias(&self.b);
        y
    }
}

pub struct LayerNorm {
    g: Vec<f32>,
    b: Vec<f32>,
    eps: f32,
}

impl LayerNorm {
    fn load(st: &SafeTensors, prefix: &str, eps: f32) -> Self {
        Self {
            g: get(st, &format!("{prefix}.weight")).1,
            b: get(st, &format!("{prefix}.bias")).1,
            eps,
        }
    }
    fn apply(&self, x: &mut Mat) {
        for r in 0..x.rows {
            layer_norm(x.row_mut(r), &self.g, &self.b, self.eps);
        }
    }
}

pub struct Layer {
    q: Linear,
    k: Linear,
    v: Linear,
    o: Linear,
    ln1: LayerNorm,
    ff1: Linear,
    ff2: Linear,
    ln2: LayerNorm,
}

impl Layer {
    fn load(st: &SafeTensors, i: usize, eps: f32) -> Self {
        let p = format!("encoder.layer.{i}");
        Self {
            q: Linear::load(st, &format!("{p}.attention.self.query")),
            k: Linear::load(st, &format!("{p}.attention.self.key")),
            v: Linear::load(st, &format!("{p}.attention.self.value")),
            o: Linear::load(st, &format!("{p}.attention.output.dense")),
            ln1: LayerNorm::load(st, &format!("{p}.attention.output.LayerNorm"), eps),
            ff1: Linear::load(st, &format!("{p}.intermediate.dense")),
            ff2: Linear::load(st, &format!("{p}.output.dense")),
            ln2: LayerNorm::load(st, &format!("{p}.output.LayerNorm"), eps),
        }
    }
    fn forward(&self, x: &Mat, cfg: &Config) -> Mat {
        let n = x.rows;
        let d = cfg.hidden / cfg.heads;
        let scale = 1.0 / (d as f32).sqrt();

        let q = self.q.forward(x);
        let k = self.k.forward(x);
        let v = self.v.forward(x);

        let mut ctx = Mat::zeros(n, cfg.hidden);
        let mut scores = vec![0.0f32; n];
        for h in 0..cfg.heads {
            let off = h * d;
            for i in 0..n {
                let qi = &q.row(i)[off..off + d];
                for j in 0..n {
                    scores[j] = dot(qi, &k.row(j)[off..off + d]) * scale;
                }
                softmax(&mut scores);
                let out = &mut ctx.row_mut(i)[off..off + d];
                for j in 0..n {
                    let vj = &v.row(j)[off..off + d];
                    let s = scores[j];
                    for t in 0..d {
                        out[t] += s * vj[t];
                    }
                }
            }
        }

        let mut a = self.o.forward(&ctx);
        a.add(x);
        self.ln1.apply(&mut a);

        let mut f = self.ff1.forward(&a);
        f.map_inplace(gelu);
        let mut y = self.ff2.forward(&f);
        y.add(&a);
        self.ln2.apply(&mut y);
        y
    }
}

pub struct Bert {
    cfg: Config,
    word_emb: Mat,
    pos_emb: Mat,
    type_emb: Vec<f32>,
    emb_ln: LayerNorm,
    layers: Vec<Layer>,
}

impl Bert {
    pub fn load(st: &SafeTensors, cfg: Config) -> Self {
        let (ws, w) = get(st, "embeddings.word_embeddings.weight");
        let (ps, p) = get(st, "embeddings.position_embeddings.weight");
        let (_, t) = get(st, "embeddings.token_type_embeddings.weight");
        Self {
            word_emb: Mat::from_vec(ws[0], ws[1], w),
            pos_emb: Mat::from_vec(ps[0], ps[1], p),
            type_emb: t[..cfg.hidden].to_vec(),
            emb_ln: LayerNorm::load(st, "embeddings.LayerNorm", cfg.eps),
            layers: (0..cfg.layers)
                .map(|i| Layer::load(st, i, cfg.eps))
                .collect(),
            cfg,
        }
    }

    pub fn forward(&self, ids: &[u32]) -> Mat {
        let h = self.cfg.hidden;
        let mut x = Mat::zeros(ids.len(), h);
        for (i, &id) in ids.iter().enumerate() {
            let row = x.row_mut(i);
            let we = self.word_emb.row(id as usize);
            let pe = self.pos_emb.row(i);
            for t in 0..h {
                row[t] = we[t] + pe[t] + self.type_emb[t];
            }
        }

        self.emb_ln.apply(&mut x);
        for layer in &self.layers {
            x = layer.forward(&x, &self.cfg);
        }
        x
    }

    pub fn embed(&self, ids: &[u32]) -> Vec<f32> {
        let hs = self.forward(ids);
        let mut out = match self.cfg.pooling {
            Pooling::Cls => hs.row(0).to_vec(),
            Pooling::Mean => {
                let mut v = vec![0.0f32; hs.cols];
                for r in 0..hs.rows {
                    for (a, b) in v.iter_mut().zip(hs.row(r)) {
                        *a += b;
                    }
                }
                let n = hs.rows as f32;
                v.iter_mut().for_each(|a| *a /= n);
                v
            }
        };

        l2_normalize(&mut out);
        out
    }
}

fn get(st: &SafeTensors, name: &str) -> (Vec<usize>, Vec<f32>) {
    if st.contains(name) {
        st.tensor(name)
    } else {
        st.tensor(&format!("bert.{name}"))
    }
}
