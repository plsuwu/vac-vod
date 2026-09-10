__device__ float block_sum(float x, float *red) {
    red[threadIdx.x] = x;
    __syncthreads();

    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (threadIdx.x < s) {
            red[threadIdx.x] += red[threadIdx.x + s];
        }
        __syncthreads();
    }

    float r = red[0];
    __syncthreads();

    return r;
}

__device__ float block_max(float x, float *red) {
    red[threadIdx.x] = x;
    __syncthreads();

    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (threadIdx.x < s) {
            red[threadIdx.x] = fmaxf(red[threadIdx.x], red[threadIdx.x + s]);
        }
        __syncthreads();
    }

    float r = red[0];
    __syncthreads();

    return r;
}

extern "C" __global__ void embed(
    const float *word,
    const float *pos,
    const float *type0,
    const int *ids,
    float *out,
    int L,
    int H
) {
    int row = blockIdx.x;
    int i = row % L;
    int id = ids[row];
    for (int t = threadIdx.x; t < H; t += blockDim.x) {
        out[(size_t)row * H + t] =
            word[(size_t)id * H + t] + pos[(size_t)i * H + t] + type0[t];
    }
}

#define TILE 32

extern "C" __global__ void gemm_t(
    const float *A,
    const float *B,
    const float *bias,
    float *C,
    int n,
    int m,
    int k,
    int act
) {
    __shared__ float As[TILE][TILE + 1];
    __shared__ float Bs[TILE][TILE + 1];
    int tx = threadIdx.x, ty = threadIdx.y;
    int row = blockIdx.y * TILE + ty; 
    int col = blockIdx.x * TILE + tx; 
    int brow = blockIdx.x * TILE + ty;
    float acc = 0.f;
    for (int t0 = 0; t0 < k; t0 += TILE) {
        As[ty][tx] =
            (row < n && t0 + tx < k) ? A[(size_t)row * k + t0 + tx] : 0.f;
        Bs[ty][tx] =
            (brow < m && t0 + tx < k) ? B[(size_t)brow * k + t0 + tx] : 0.f;
        __syncthreads();
#pragma unroll
        for (int t = 0; t < TILE; t++) {
            acc += As[ty][t] * Bs[tx][t];
        }
        __syncthreads();
    }
    if (row < n && col < m) {
        float v = acc + bias[col];
        if (act == 1) {
            v = 0.5f * v * (1.f + erff(v * 0.70710678f));
        }

        C[(size_t)row * m + col] = v;
    }
}

extern "C" __global__ void add_layernorm(
    const float *a,
    const float *b,
    const float *g,
    const float *beta,
    float *out,
    int H,
    float eps
) {
    extern __shared__ float sh[];

    float *v = sh;
    float *red = sh + H;
    size_t base = (size_t)blockIdx.x * H;
    float s = 0.f;

    for (int t = threadIdx.x; t < H; t += blockDim.x) {
        float x = a[base + t] + b[base + t];
        v[t] = x;
        s += x;
    }

    float mean = block_sum(s, red) / H;
    float s2 = 0.f;

    for (int t = threadIdx.x; t < H; t += blockDim.x) {
        float d = v[t] - mean;
        s2 += d * d;
    }

    float inv = rsqrtf(block_sum(s2, red) / H + eps);
    for (int t = threadIdx.x; t < H; t += blockDim.x) {
        out[base + t] = (v[t] - mean) * inv * g[t] + beta[t];
    }
}

extern "C" __global__ void attention(
    const float *q,
    const float *k,
    const float *v,
    const float *mask,
    float *out,
    int L,
    int H,
    int heads
) {
    extern __shared__ float sh[];

    float *sc = sh;
    float *red = sh + L;
    int i = blockIdx.x, h = blockIdx.y, b = blockIdx.z;
    int d = H / heads, off = h * d;
    const float *qi = q + ((size_t)(b * L + i)) * H + off;
    float scale = rsqrtf((float)d);
    float mx = -1e30f;

    for (int j = threadIdx.x; j < L; j += blockDim.x) {
        float s = -1e30f;
        if (mask[b * L + j] > 0.5f) {
            const float *kj = k + ((size_t)(b * L + j)) * H + off;
            s = 0.f;
            for (int t = 0; t < d; t++) {
                s += qi[t] * kj[t];
            }
            s *= scale;
        }
        sc[j] = s;
        mx = fmaxf(mx, s);
    }

    mx = block_max(mx, red);
    float sum = 0.f;

    for (int j = threadIdx.x; j < L; j += blockDim.x) {
        float e = __expf(sc[j] - mx);
        sc[j] = e;
        sum += e;
    }
    float inv = 1.f / block_sum(sum, red);

    for (int t = threadIdx.x; t < d; t += blockDim.x) {
        float acc = 0.f;
        for (int j = 0; j < L; j++) {
            acc += sc[j] * v[((size_t)(b * L + j)) * H + off + t];
        }
        out[((size_t)(b * L + i)) * H + off + t] = acc * inv;
    }
}

extern "C" __global__ void pool_norm(
    const float *x,
    const float *mask,
    float *out,
    int L,
    int H,
    int cls
) {
    extern __shared__ float red[];
    int b = blockIdx.x;
    float ss = 0.f;
    for (int t = threadIdx.x; t < H; t += blockDim.x) {
        float v;
        if (cls) {
            v = x[((size_t)b * L) * H + t];
        } else {
            float s = 0.f, c = 0.f;
            for (int j = 0; j < L; j++) {
                float m = mask[b * L + j];
                s += m * x[((size_t)(b * L + j)) * H + t];
                c += m;
            }
            v = s / c;
        }
        out[(size_t)b * H + t] = v;
        ss += v * v;
    }
    ss = block_sum(ss, red);
    float inv = rsqrtf(fmaxf(ss, 1e-24f));
    for (int t = threadIdx.x; t < H; t += blockDim.x) {
        out[(size_t)b * H + t] *= inv;
    }
}
