use std::error::Error;
use std::sync::Arc;

use cudarc::driver::{
    CudaContext, CudaFunction, CudaSlice, CudaStream, LaunchConfig, PushKernelArg,
};
use cudarc::nvrtc::compile_ptx;
use tokio::sync::mpsc::UnboundedSender;

use crate::bert::{Config, Pooling};
use crate::safetensors::SafeTensors;

const KERNELS: &str = include_str!("kernels.cu");
const BLOCK: u32 = 128;

type Res<T> = Result<T, Box<dyn Error>>;

struct GpuLinear {
    w: CudaSlice<f32>, // [out, in]
    b: CudaSlice<f32>,
    out: usize,
    inp: usize,
}

struct GpuLayerNorm {
    g: CudaSlice<f32>,
    b: CudaSlice<f32>,
}

struct GpuLayer {
    q: GpuLinear,
    k: GpuLinear,
    v: GpuLinear,
    o: GpuLinear,
    ln1: GpuLayerNorm,
    ff1: GpuLinear,
    ff2: GpuLinear,
    ln2: GpuLayerNorm,
}

pub struct GpuBert {
    cfg: Config,
    stream: Arc<CudaStream>,
    k_embed: CudaFunction,
    k_gemm: CudaFunction,
    k_ln: CudaFunction,
    k_attn: CudaFunction,
    k_pool: CudaFunction,
    word_emb: CudaSlice<f32>,
    pos_emb: CudaSlice<f32>,
    type_emb: CudaSlice<f32>,
    emb_ln: GpuLayerNorm,
    layers: Vec<GpuLayer>,
}

fn get(st: &SafeTensors, name: &str) -> (Vec<usize>, Vec<f32>) {
    if st.contains(name) {
        st.tensor(name)
    } else {
        st.tensor(&format!("bert.{name}"))
    }
}

fn div_up(a: usize, b: u32) -> u32 {
    (a as u32).div_ceil(b)
}

impl GpuBert {
    pub fn load(st: &SafeTensors, cfg: Config) -> Res<Self> {
        let ctx = CudaContext::new(0)?;
        let stream = ctx.default_stream();

        let ptx = compile_ptx(KERNELS)?;
        let module = ctx.load_module(ptx)?;
        let func = |n: &str| module.load_function(n);

        let up = |name: &str| -> Res<CudaSlice<f32>> { Ok(stream.clone_htod(&get(st, name).1)?) };
        let lin = |p: &str| -> Res<GpuLinear> {
            let (shape, w) = get(st, &format!("{p}.weight"));
            Ok(GpuLinear {
                w: stream.clone_htod(&w)?,
                b: up(&format!("{p}.bias"))?,
                out: shape[0],
                inp: shape[1],
           })
        };
        let ln = |p: &str| -> Res<GpuLayerNorm> {
            Ok(GpuLayerNorm {
                g: up(&format!("{p}.weight"))?,
                b: up(&format!("{p}.bias"))?,
            })
        };

        let mut layers = Vec::with_capacity(cfg.layers);
        for i in 0..cfg.layers {
            let p = format!("encoder.layer.{i}");
            layers.push(GpuLayer {
                q: lin(&format!("{p}.attention.self.query"))?,
                k: lin(&format!("{p}.attention.self.key"))?,
                v: lin(&format!("{p}.attention.self.value"))?,
                o: lin(&format!("{p}.attention.output.dense"))?,
                ln1: ln(&format!("{p}.attention.output.LayerNorm"))?,
                ff1: lin(&format!("{p}.intermediate.dense"))?,
                ff2: lin(&format!("{p}.output.dense"))?,
                ln2: ln(&format!("{p}.output.LayerNorm"))?,
            });
        }

        let type_row = get(st, "embeddings.token_type_embeddings.weight").1[..cfg.hidden].to_vec();

        Ok(Self {
            cfg,
            k_embed: func("embed")?,
            k_gemm: func("gemm_t")?,
            k_ln: func("add_layernorm")?,
            k_attn: func("attention")?,
            k_pool: func("pool_norm")?,
            word_emb: up("embeddings.word_embeddings.weight")?,
            pos_emb: up("embeddings.position_embeddings.weight")?,
            type_emb: stream.clone_htod(&type_row)?,
            emb_ln: ln("embeddings.LayerNorm")?,
            layers,
            stream,
        })
    }

    pub fn embed_batch(&self, seqs: &[Vec<u32>]) -> Res<Vec<Vec<f32>>> {
        let bsz = seqs.len();
        let l = seqs.iter().map(Vec::len).max().unwrap_or(0);
        let h = self.cfg.hidden;
        let rows = bsz * l;

        let mut ids = vec![0i32; rows];
        let mut mask = vec![0f32; rows];
        for (b, s) in seqs.iter().enumerate() {
            for (i, &id) in s.iter().enumerate() {
                ids[b * l + i] = id as i32;
                mask[b * l + i] = 1.0;
            }
        }
        let ids_d = self.stream.clone_htod(&ids)?;
        let mask_d = self.stream.clone_htod(&mask)?;

        let s = &self.stream;
        let mut x = s.alloc_zeros::<f32>(rows * h)?;
        let mut q = s.alloc_zeros::<f32>(rows * h)?;
        let mut k = s.alloc_zeros::<f32>(rows * h)?;
        let mut v = s.alloc_zeros::<f32>(rows * h)?;
        let mut ctx = s.alloc_zeros::<f32>(rows * h)?;
        let mut tmp = s.alloc_zeros::<f32>(rows * h)?;
        let mut a = s.alloc_zeros::<f32>(rows * h)?;
        let mut f = s.alloc_zeros::<f32>(rows * self.cfg.intermediate)?;
        let mut out = s.alloc_zeros::<f32>(bsz * h)?;

        // embeddings
        let (li, hi) = (l as i32, h as i32);
        {
            let mut b = s.launch_builder(&self.k_embed);
            b.arg(&self.word_emb);
            b.arg(&self.pos_emb);
            b.arg(&self.type_emb);
            b.arg(&ids_d);
            b.arg(&mut tmp);
            b.arg(&li);
            b.arg(&hi);
            unsafe {
                b.launch(LaunchConfig {
                    grid_dim: (rows as u32, 1, 1),
                    block_dim: (BLOCK, 1, 1),
                    shared_mem_bytes: 0,
                })
            }?;
        }

        self.add_ln(&tmp, &a, &self.emb_ln, &mut x)?;

        for layer in &self.layers {
            self.gemm(&x, rows, &layer.q, &mut q, false)?;
            self.gemm(&x, rows, &layer.k, &mut k, false)?;
            self.gemm(&x, rows, &layer.v, &mut v, false)?;
            self.attention(&q, &k, &v, &mask_d, &mut ctx, bsz, l)?;
            self.gemm(&ctx, rows, &layer.o, &mut tmp, false)?;
            self.add_ln(&tmp, &x, &layer.ln1, &mut a)?; // a = LN(o(ctx) + x)
            self.gemm(&a, rows, &layer.ff1, &mut f, true)?; // GELU fused
            self.gemm(&f, rows, &layer.ff2, &mut tmp, false)?;
            self.add_ln(&tmp, &a, &layer.ln2, &mut x)?; // x = LN(ff2(f) + a)
        }

        let cls = matches!(self.cfg.pooling, Pooling::Cls) as i32;
        {
            let mut b = s.launch_builder(&self.k_pool);
            b.arg(&x);
            b.arg(&mask_d);
            b.arg(&mut out);
            b.arg(&li);
            b.arg(&hi);
            b.arg(&cls);
            unsafe {
                b.launch(LaunchConfig {
                    grid_dim: (bsz as u32, 1, 1),
                    block_dim: (BLOCK, 1, 1),
                    shared_mem_bytes: BLOCK * 4,
                })
            }?;
        }

        let host = s.clone_dtoh(&out)?;
        Ok(host.chunks(h).map(<[f32]>::to_vec).collect())
    }

    fn gemm(
        &self,
        inp: &CudaSlice<f32>,
        n: usize,
        lin: &GpuLinear,
        out: &mut CudaSlice<f32>,
        gelu: bool,
    ) -> Res<()> {
        let (ni, mi, ki, act) = (n as i32, lin.out as i32, lin.inp as i32, gelu as i32);
        let cfg = LaunchConfig {
            grid_dim: (div_up(lin.out, 32), div_up(n, 32), 1),
            block_dim: (32, 32, 1),
            shared_mem_bytes: 0,
        };
        let mut b = self.stream.launch_builder(&self.k_gemm);
        b.arg(inp);
        b.arg(&lin.w);
        b.arg(&lin.b);
        b.arg(out);
        b.arg(&ni);
        b.arg(&mi);
        b.arg(&ki);
        b.arg(&act);
        unsafe { b.launch(cfg) }?;
        Ok(())
    }

    fn add_ln(
        &self,
        x: &CudaSlice<f32>,
        resid: &CudaSlice<f32>,
        ln: &GpuLayerNorm,
        out: &mut CudaSlice<f32>,
    ) -> Res<()> {
        let h = self.cfg.hidden;
        let rows = (x.len() / h) as u32;
        let (hi, eps) = (h as i32, self.cfg.eps);
        let cfg = LaunchConfig {
            grid_dim: (rows, 1, 1),
            block_dim: (BLOCK, 1, 1),
            shared_mem_bytes: (h as u32 + BLOCK) * 4,
        };
        let mut b = self.stream.launch_builder(&self.k_ln);
        b.arg(x);
        b.arg(resid);
        b.arg(&ln.g);
        b.arg(&ln.b);
        b.arg(out);
        b.arg(&hi);
        b.arg(&eps);
        unsafe { b.launch(cfg) }?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn attention(
        &self,
        q: &CudaSlice<f32>,
        k: &CudaSlice<f32>,
        v: &CudaSlice<f32>,
        mask: &CudaSlice<f32>,
        out: &mut CudaSlice<f32>,
        bsz: usize,
        l: usize,
    ) -> Res<()> {
        let (li, hi, heads) = (l as i32, self.cfg.hidden as i32, self.cfg.heads as i32);
        let cfg = LaunchConfig {
            grid_dim: (l as u32, self.cfg.heads as u32, bsz as u32),
            block_dim: (BLOCK, 1, 1),
            shared_mem_bytes: (l as u32 + BLOCK) * 4,
        };
        let mut b = self.stream.launch_builder(&self.k_attn);
        b.arg(q);
        b.arg(k);
        b.arg(v);
        b.arg(mask);
        b.arg(out);
        b.arg(&li);
        b.arg(&hi);
        b.arg(&heads);
        unsafe { b.launch(cfg) }?;
        Ok(())
    }
}
